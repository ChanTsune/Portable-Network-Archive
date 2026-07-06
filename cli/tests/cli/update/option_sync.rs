use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: An archive contains multiple files.
/// Action: Delete a file from source, run `pna experimental update` without `--sync`.
/// Expectation: Entries for deleted source files are preserved in the archive.
#[test]
fn update_without_sync() {
    setup();
    TestResources::extract_in("raw/", "update_without_sync/in/").unwrap();

    // Create initial archive with all files
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_without_sync/archive.pna",
        "--overwrite",
        "update_without_sync/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify initial archive contains empty.txt
    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_without_sync/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert!(
        initial_entries.iter().any(|p| p.ends_with("raw/empty.txt")),
        "initial archive should contain empty.txt"
    );
    let initial_count = initial_entries.len();

    // Delete empty.txt from source
    fs::remove_file("update_without_sync/in/raw/empty.txt").unwrap();

    // Run update command WITHOUT --sync
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_without_sync/archive.pna",
        "update_without_sync/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents after update
    let mut seen = HashSet::new();
    archive::for_each_entry("update_without_sync/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Verify empty.txt is STILL in the archive (default: preserve deleted entries)
    assert!(
        seen.iter().any(|p| p.ends_with("raw/empty.txt")),
        "empty.txt should be preserved in the archive without --sync"
    );

    // Verify other files are still present
    assert!(
        seen.iter().any(|p| p.ends_with("raw/text.txt")),
        "text.txt should still be in the archive"
    );

    // Verify entry count hasn't changed
    assert_eq!(
        seen.len(),
        initial_count,
        "archive entry count should remain the same without --sync"
    );
}

/// Precondition: An archive contains multiple files.
/// Action: Delete a file from source, run `pna experimental update` with `--sync`.
/// Expectation: Entries for deleted source files are removed from the archive.
#[test]
fn update_with_sync() {
    setup();
    TestResources::extract_in("raw/", "update_with_sync/in/").unwrap();

    // Create initial archive with all files
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_with_sync/archive.pna",
        "--overwrite",
        "update_with_sync/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify initial archive contains empty.txt
    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_with_sync/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert!(
        initial_entries.iter().any(|p| p.ends_with("raw/empty.txt")),
        "initial archive should contain empty.txt"
    );
    let initial_count = initial_entries.len();

    // Delete empty.txt from source
    fs::remove_file("update_with_sync/in/raw/empty.txt").unwrap();

    // Run update command WITH --sync
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_with_sync/archive.pna",
        "update_with_sync/in/",
        "--keep-timestamp",
        "--sync",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents after update
    let mut seen = HashSet::new();
    archive::for_each_entry("update_with_sync/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Verify empty.txt was REMOVED from the archive
    assert!(
        !seen.iter().any(|p| p.ends_with("raw/empty.txt")),
        "empty.txt should be removed from the archive with --sync, but found entries: {seen:?}"
    );

    // Verify other files are still present
    assert!(
        seen.iter().any(|p| p.ends_with("raw/text.txt")),
        "text.txt should still be in the archive"
    );

    // Verify entry count decreased by 1
    assert_eq!(
        seen.len(),
        initial_count - 1,
        "archive entry count should decrease by 1 with --sync"
    );

    // Extract and verify the extracted output doesn't contain the deleted file
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "update_with_sync/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_with_sync/out/",
        "--keep-timestamp",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify empty.txt does not exist in extracted output
    assert!(
        !std::path::Path::new("update_with_sync/out/raw/empty.txt").exists(),
        "empty.txt should not exist in extracted output"
    );

    // Verify text.txt exists in extracted output
    assert!(
        std::path::Path::new("update_with_sync/out/raw/text.txt").exists(),
        "text.txt should exist in extracted output"
    );
}

/// Precondition: An archive contains multiple files, all of which exist on disk.
/// Action: Run `pna experimental update --sync` with `--exclude` matching one file.
/// Expectation: The excluded entry is kept; --sync prunes only entries whose
/// files no longer exist on disk, not entries filtered out of collection.
#[test]
fn update_with_sync_keeps_excluded_but_existing_file() {
    setup();
    TestResources::extract_in("raw/", "update_sync_exclude_existing/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_sync_exclude_existing/archive.pna",
        "--overwrite",
        "update_sync_exclude_existing/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_sync_exclude_existing/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    // empty.txt stays on disk but is excluded from collection.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--sync",
        "-f",
        "update_sync_exclude_existing/archive.pna",
        "update_sync_exclude_existing/in/",
        "--keep-timestamp",
        "--exclude",
        "update_sync_exclude_existing/in/raw/empty.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_sync_exclude_existing/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert!(
        seen.iter().any(|p| p.ends_with("raw/empty.txt")),
        "excluded but existing empty.txt should be kept under --sync"
    );
    assert_eq!(
        seen, initial_entries,
        "no entry should be pruned when all files exist on disk"
    );
}

/// Precondition: An archive contains multiple files, all of which exist on disk.
/// Action: Run `pna experimental update --sync` with a future `--newer-mtime`
/// filter so that no file is collected.
/// Expectation: All entries are kept; time-filtered files still exist on disk.
#[test]
fn update_with_sync_keeps_time_filtered_files() {
    setup();
    TestResources::extract_in("raw/", "update_sync_time_filtered/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_sync_time_filtered/archive.pna",
        "--overwrite",
        "update_sync_time_filtered/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_sync_time_filtered/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    // The far-future threshold filters every file out of collection.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--sync",
        "-f",
        "update_sync_time_filtered/archive.pna",
        "update_sync_time_filtered/in/",
        "--keep-timestamp",
        "--newer-mtime",
        "@4102444800",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_sync_time_filtered/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(
        seen, initial_entries,
        "time-filtered but existing files should be kept under --sync"
    );
}
