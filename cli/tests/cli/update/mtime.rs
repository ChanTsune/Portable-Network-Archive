use crate::utils::{archive::for_each_entry, setup, EmbedExt, TestResources};
use clap::Parser;
use pna::Duration;
use portable_network_archive::{cli, command::Command};
use std::{fs, io::prelude::*, time::SystemTime};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);

#[test]
fn archive_update_with_mtime() {
    setup();
    TestResources::extract_in("raw/", "archive_update_with_mtime/in/").unwrap();
    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_update_with_mtime/update_with_mtime.pna",
        "--overwrite",
        "archive_update_with_mtime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer mtime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_with_mtime/in/raw/text.txt")
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
        "archive_update_with_mtime/update_with_mtime.pna",
        "archive_update_with_mtime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify mtime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("archive_update_with_mtime/update_with_mtime.pna", |entry| {
        assert_eq!(entry.metadata().modified(), Some(expected));
    })
    .unwrap();
}

#[test]
fn archive_update_with_clamp_mtime() {
    setup();
    TestResources::extract_in("raw/", "archive_update_with_clamp_mtime/in/").unwrap();
    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_update_with_clamp_mtime/update_with_clamp_mtime.pna",
        "--overwrite",
        "archive_update_with_clamp_mtime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer mtime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_with_clamp_mtime/in/raw/text.txt")
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
        "archive_update_with_clamp_mtime/update_with_clamp_mtime.pna",
        "archive_update_with_clamp_mtime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify mtime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry(
        "archive_update_with_clamp_mtime/update_with_clamp_mtime.pna",
        |entry| {
            assert!(entry.metadata().modified() <= Some(expected));
        },
    )
    .unwrap();
}
