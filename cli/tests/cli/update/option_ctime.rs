use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{fs, io::prelude::*, time::SystemTime};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);

/// Precondition: An archive contains files.
/// Action: Modify a file, run `pna experimental update` with `--ctime`.
/// Expectation: All entries in the archive have the specified ctime.
#[test]
fn update_with_ctime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_with_ctime");
    TestResources::extract_in("raw/", "update_with_ctime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_with_ctime/archive.pna",
        "--overwrite",
        "update_with_ctime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer ctime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_ctime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update archive with specified ctime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--ctime",
        "2024-01-01T00:00:00Z",
        "-f",
        "update_with_ctime/archive.pna",
        "update_with_ctime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("update_with_ctime/archive.pna", |entry| {
        assert_eq!(entry.metadata().created(), Some(expected));
    })
    .unwrap();
}

/// Precondition: An archive contains files.
/// Action: Modify a file, run `pna experimental update` with `--ctime` and `--clamp-ctime`.
/// Expectation: All entries in the archive have ctime clamped to the specified value.
#[test]
fn update_with_clamp_ctime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_with_clamp_ctime");
    TestResources::extract_in("raw/", "update_with_clamp_ctime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_with_clamp_ctime/archive.pna",
        "--overwrite",
        "update_with_clamp_ctime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer ctime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_clamp_ctime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update archive with specified ctime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--ctime",
        "2024-01-01T00:00:00Z",
        "--clamp-ctime",
        "-f",
        "update_with_clamp_ctime/archive.pna",
        "update_with_clamp_ctime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("update_with_clamp_ctime/archive.pna", |entry| {
        assert!(entry.metadata().created() <= Some(expected));
    })
    .unwrap();
}
