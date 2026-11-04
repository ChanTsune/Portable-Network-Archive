use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

fn create_directory_archive(dir: &str) {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(format!("{dir}/in/sub")).unwrap();
    fs::write(format!("{dir}/in/keep.txt"), b"keep content").unwrap();
    fs::write(format!("{dir}/in/sub/inner.txt"), b"inner content").unwrap();

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

/// Precondition: An archive contains a directory entry and its contained
/// file, then the directory is removed from disk.
/// Action: Run `pna experimental update` with `--sync`.
/// Expectation: The directory entry and its contained file entry are pruned
/// while other entries remain.
#[test]
fn update_with_sync_prunes_missing_directory() {
    setup();
    create_directory_archive("update_sync_missing_dir");

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_sync_missing_dir/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    fs::remove_dir_all("update_sync_missing_dir/in/sub").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--sync",
        "-f",
        "update_sync_missing_dir/archive.pna",
        "update_sync_missing_dir/in",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_sync_missing_dir/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    let mut expected = initial_entries;
    assert!(
        expected.remove("update_sync_missing_dir/in/sub"),
        "precondition: initial archive should contain the directory entry"
    );
    assert!(
        expected.remove("update_sync_missing_dir/in/sub/inner.txt"),
        "precondition: initial archive should contain the contained file entry"
    );
    assert_eq!(
        seen, expected,
        "the directory missing on disk and its contents should be pruned"
    );
}

/// Precondition: An archive contains a directory entry and its contained
/// file, then the directory is removed from disk.
/// Action: Run `pna experimental update` without `--sync`.
/// Expectation: All entries including the removed directory are preserved.
#[test]
fn update_without_sync_keeps_missing_directory() {
    setup();
    create_directory_archive("update_no_sync_missing_dir");

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_no_sync_missing_dir/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    fs::remove_dir_all("update_no_sync_missing_dir/in/sub").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_no_sync_missing_dir/archive.pna",
        "update_no_sync_missing_dir/in",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_no_sync_missing_dir/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(
        seen, initial_entries,
        "entries for removed directories should be preserved without --sync"
    );
}
