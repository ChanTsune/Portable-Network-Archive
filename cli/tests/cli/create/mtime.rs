use crate::utils::{archive::for_each_entry, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs,
    io::prelude::*,
    time::{Duration, SystemTime},
};

const DURATION_24_HOURS: Duration = Duration::from_secs(24 * 60 * 60);

#[test]
fn archive_create_with_mtime() {
    setup();
    TestResources::extract_in("raw/", "archive_create_with_mtime/in/").unwrap();

    // Update file with newer mtime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_create_with_mtime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Create archive with specified mtime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_create_with_mtime/create_with_mtime.pna",
        "--overwrite",
        "archive_create_with_mtime/in/",
        "--keep-timestamp",
        "--mtime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify mtime is set correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry("archive_create_with_mtime/create_with_mtime.pna", |entry| {
        assert_eq!(entry.metadata().modified(), Some(expected));
    })
    .unwrap();
}

#[test]
fn archive_create_with_clamp_mtime() {
    setup();
    TestResources::extract_in("raw/", "archive_create_with_clamp_mtime/in/").unwrap();

    // Update file with newer mtime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_create_with_clamp_mtime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Create archive with specified mtime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_create_with_clamp_mtime/create_with_clamp_mtime.pna",
        "--overwrite",
        "archive_create_with_clamp_mtime/in/",
        "--keep-timestamp",
        "--mtime",
        "2024-01-01T00:00:00Z",
        "--clamp-mtime",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify mtime is clamped correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry(
        "archive_create_with_clamp_mtime/create_with_clamp_mtime.pna",
        |entry| {
            assert!(entry.metadata().modified() <= Some(expected));
        },
    )
    .unwrap();
}
