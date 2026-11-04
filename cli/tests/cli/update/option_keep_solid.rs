use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, io::prelude::*, time};

const DURATION_24_HOURS: time::Duration = time::Duration::from_secs(24 * 60 * 60);

/// Precondition: A solid mode archive exists and a source file is modified
/// with a newer mtime.
/// Action: Run `pna experimental update` with `--keep-solid`.
/// Expectation: The solid group is preserved as a solid entry and the updated
/// file is appended as a normal entry (mixed mode archive). The entry path set
/// is unchanged and extraction yields the updated content.
#[test]
fn update_with_keep_solid() {
    setup();
    TestResources::extract_in("raw/", "update_with_keep_solid/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_with_keep_solid/archive.pna",
        "--overwrite",
        "--solid",
        "--no-keep-dir",
        "update_with_keep_solid/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_with_keep_solid/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_keep_solid/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated content for keep-solid test")
        .unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--keep-solid",
        "-f",
        "update_with_keep_solid/archive.pna",
        "update_with_keep_solid/in/",
        "--no-keep-dir",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut archive = pna::Archive::open("update_with_keep_solid/archive.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| matches!(entry, pna::ReadEntry::Solid(_)))
            .count(),
        1,
        "the solid group should be preserved as a single solid entry"
    );
    assert!(
        entries
            .iter()
            .any(|entry| matches!(entry, pna::ReadEntry::Normal(_))),
        "the updated file should be appended as a normal entry"
    );

    let mut seen = HashSet::new();
    archive::for_each_entry("update_with_keep_solid/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert_eq!(
        seen, initial_entries,
        "entry path set should be unchanged after update"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "update_with_keep_solid/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_with_keep_solid/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert_eq!(
        fs::read("update_with_keep_solid/out/raw/text.txt").unwrap(),
        b"updated content for keep-solid test",
        "extraction should yield the updated content"
    );
}

/// Precondition: A solid mode archive exists and a source file is deleted
/// from disk.
/// Action: Run `pna experimental update` with `--keep-solid` and `--sync`.
/// Expectation: The entry whose file is missing on disk is pruned from inside
/// the solid group, and the result remains a single solid entry.
#[test]
fn update_with_keep_solid_and_sync() {
    setup();
    TestResources::extract_in("raw/", "update_keep_solid_sync/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_keep_solid_sync/archive.pna",
        "--overwrite",
        "--solid",
        "--no-keep-dir",
        "update_keep_solid_sync/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_keep_solid_sync/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    fs::remove_file("update_keep_solid_sync/in/raw/empty.txt").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--keep-solid",
        "--sync",
        "-f",
        "update_keep_solid_sync/archive.pna",
        "update_keep_solid_sync/in/",
        "--no-keep-dir",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut archive = pna::Archive::open("update_keep_solid_sync/archive.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, pna::ReadEntry::Solid(_))),
        "all entries should remain in solid mode"
    );
    assert_eq!(entries.len(), 1);

    let mut seen = HashSet::new();
    archive::for_each_entry("update_keep_solid_sync/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    let mut expected = initial_entries;
    assert!(
        expected.remove("update_keep_solid_sync/in/raw/empty.txt"),
        "precondition: initial archive should contain the deleted file"
    );
    assert_eq!(
        seen, expected,
        "only the entry missing on disk should be pruned from the solid group"
    );
}
