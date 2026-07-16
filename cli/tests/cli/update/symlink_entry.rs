use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, path::Path};

fn create_symlink_archive(dir: &str) {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(format!("{dir}/in")).unwrap();
    fs::write(format!("{dir}/in/target.txt"), b"target content").unwrap();
    pna::fs::symlink(Path::new("target.txt"), format!("{dir}/in/link.txt")).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &format!("{dir}/archive.pna"),
        "--overwrite",
        &format!("{dir}/in"),
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();
}

/// Precondition: An archive contains a symlink entry and the source tree is
/// unchanged.
/// Action: Run `pna experimental update`.
/// Expectation: The symlink entry is preserved with its symlink kind and the
/// entry set is unchanged.
#[test]
fn update_keeps_symlink_entry() {
    setup();
    create_symlink_archive("update_keeps_symlink");

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_keeps_symlink/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_keeps_symlink/archive.pna",
        "update_keeps_symlink/in",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    let mut symlink_kind = false;
    archive::for_each_entry("update_keeps_symlink/archive.pna", |entry| {
        if entry.header().path().as_str() == "update_keeps_symlink/in/link.txt" {
            symlink_kind = entry.header().data_kind() == pna::DataKind::SYMBOLIC_LINK;
        }
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert!(
        symlink_kind,
        "link.txt should remain a symlink entry after update"
    );
    assert_eq!(
        seen, initial_entries,
        "entry set should be unchanged after update"
    );
}

/// Precondition: An archive contains a symlink entry and the symlink is
/// removed from disk.
/// Action: Run `pna experimental update` with `--sync`.
/// Expectation: The symlink entry is pruned while the target file entry
/// remains.
#[test]
fn update_with_sync_prunes_missing_symlink() {
    setup();
    create_symlink_archive("update_sync_missing_symlink");

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_sync_missing_symlink/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    fs::remove_file("update_sync_missing_symlink/in/link.txt").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--sync",
        "-f",
        "update_sync_missing_symlink/archive.pna",
        "update_sync_missing_symlink/in",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_sync_missing_symlink/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    let mut expected = initial_entries;
    assert!(
        expected.remove("update_sync_missing_symlink/in/link.txt"),
        "precondition: initial archive should contain the symlink entry"
    );
    assert_eq!(
        seen, expected,
        "only the symlink missing on disk should be pruned"
    );
}

/// Precondition: An archive contains a symlink entry whose target is removed
/// from disk, leaving the symlink itself broken but present.
/// Action: Run `pna experimental update` with `--sync`.
/// Expectation: The broken symlink entry is kept (the link itself exists on
/// disk) while the removed target file entry is pruned.
#[cfg(unix)]
#[test]
fn update_with_sync_keeps_broken_symlink() {
    setup();
    create_symlink_archive("update_sync_broken_symlink");

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_sync_broken_symlink/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    fs::remove_file("update_sync_broken_symlink/in/target.txt").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--sync",
        "-f",
        "update_sync_broken_symlink/archive.pna",
        "update_sync_broken_symlink/in",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_sync_broken_symlink/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert!(
        seen.contains("update_sync_broken_symlink/in/link.txt"),
        "broken symlink existing on disk should be kept under --sync"
    );
    let mut expected = initial_entries;
    assert!(
        expected.remove("update_sync_broken_symlink/in/target.txt"),
        "precondition: initial archive should contain the target file entry"
    );
    assert_eq!(
        seen, expected,
        "only the target file missing on disk should be pruned"
    );
}
