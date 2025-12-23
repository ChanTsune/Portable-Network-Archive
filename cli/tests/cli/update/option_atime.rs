use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{
    fs::{self, FileTimes},
    io::prelude::*,
    time::SystemTime,
};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);

/// Precondition: An archive contains files.
/// Action: Modify a file, run `pna experimental update` with `--atime`.
/// Expectation: All entries in the archive have the specified atime.
#[test]
fn update_with_atime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_with_atime");
    TestResources::extract_in("raw/", "update_with_atime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_with_atime/archive.pna",
        "--overwrite",
        "update_with_atime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer atime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_atime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_times(FileTimes::new().set_accessed(SystemTime::now() + DURATION_24_HOURS))
        .unwrap();

    // Update archive with specified atime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--atime",
        "2024-01-01T00:00:00Z",
        "-f",
        "update_with_atime/archive.pna",
        "update_with_atime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("update_with_atime/archive.pna", |entry| {
        assert_eq!(entry.metadata().accessed(), Some(expected));
    })
    .unwrap();
}

/// Precondition: An archive contains files.
/// Action: Modify a file, run `pna experimental update` with `--atime` and `--clamp-atime`.
/// Expectation: All entries in the archive have atime clamped to the specified value.
#[test]
fn update_with_clamp_atime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_with_clamp_atime");
    TestResources::extract_in("raw/", "update_with_clamp_atime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_with_clamp_atime/archive.pna",
        "--overwrite",
        "update_with_clamp_atime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer atime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_clamp_atime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_times(FileTimes::new().set_accessed(SystemTime::now() + DURATION_24_HOURS))
        .unwrap();

    // Update archive with specified atime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--atime",
        "2024-01-01T00:00:00Z",
        "--clamp-atime",
        "-f",
        "update_with_clamp_atime/archive.pna",
        "update_with_clamp_atime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("update_with_clamp_atime/archive.pna", |entry| {
        assert!(entry.metadata().accessed() <= Some(expected));
    })
    .unwrap();
}
