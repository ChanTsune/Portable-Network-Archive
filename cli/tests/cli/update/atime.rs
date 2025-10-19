use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup};
use clap::Parser;
use pna::Duration;
use portable_network_archive::{cli, command::Command};
use std::{
    fs::{self, FileTimes},
    io::prelude::*,
    time::SystemTime,
};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);

#[test]
fn archive_update_with_atime() {
    setup();
    TestResources::extract_in("raw/", "archive_update_with_atime/in/").unwrap();
    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_update_with_atime/update_with_atime.pna",
        "--overwrite",
        "archive_update_with_atime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer atime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_with_atime/in/raw/text.txt")
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
        "archive_update_with_atime/update_with_atime.pna",
        "archive_update_with_atime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("archive_update_with_atime/update_with_atime.pna", |entry| {
        assert_eq!(entry.metadata().accessed(), Some(expected));
    })
    .unwrap();
}

#[test]
fn archive_update_with_clamp_atime() {
    setup();
    TestResources::extract_in("raw/", "archive_update_with_clamp_atime/in/").unwrap();
    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_update_with_clamp_atime/update_with_clamp_atime.pna",
        "--overwrite",
        "archive_update_with_clamp_atime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer atime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_with_clamp_atime/in/raw/text.txt")
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
        "archive_update_with_clamp_atime/update_with_clamp_atime.pna",
        "archive_update_with_clamp_atime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry(
        "archive_update_with_clamp_atime/update_with_clamp_atime.pna",
        |entry| {
            assert!(entry.metadata().accessed() <= Some(expected));
        },
    )
    .unwrap();
}
