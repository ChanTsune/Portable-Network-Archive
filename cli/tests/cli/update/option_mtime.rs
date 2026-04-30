use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{fs, io::prelude::*, time::SystemTime};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);

/// Precondition: An archive contains files.
/// Action: Modify a file, run `pna experimental update` with `--mtime`.
/// Expectation: All entries in the archive have the specified mtime.
#[test]
fn update_with_mtime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_with_mtime");
    TestResources::extract_in("raw/", "update_with_mtime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_with_mtime/archive.pna",
        "--overwrite",
        "update_with_mtime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer mtime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_mtime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update archive with specified mtime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--mtime",
        "2024-01-01T00:00:00Z",
        "-f",
        "update_with_mtime/archive.pna",
        "update_with_mtime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify mtime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("update_with_mtime/archive.pna", |entry| {
        assert_eq!(entry.metadata().modified(), Some(expected));
    })
    .unwrap();
}

/// Precondition: An archive contains files.
/// Action: Modify a file, run `pna experimental update` with `--mtime` and `--clamp-mtime`.
/// Expectation: All entries in the archive have mtime clamped to the specified value.
#[test]
fn update_with_clamp_mtime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_with_clamp_mtime");
    TestResources::extract_in("raw/", "update_with_clamp_mtime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_with_clamp_mtime/archive.pna",
        "--overwrite",
        "update_with_clamp_mtime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer mtime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_clamp_mtime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update archive with specified mtime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--mtime",
        "2024-01-01T00:00:00Z",
        "--clamp-mtime",
        "-f",
        "update_with_clamp_mtime/archive.pna",
        "update_with_clamp_mtime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify mtime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("update_with_clamp_mtime/archive.pna", |entry| {
        assert!(entry.metadata().modified() <= Some(expected));
    })
    .unwrap();
}
