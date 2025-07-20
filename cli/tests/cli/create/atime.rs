use crate::utils::{archive::for_each_entry, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs::{self, FileTimes},
    io::prelude::*,
    time::{Duration, SystemTime},
};

const DURATION_24_HOURS: Duration = Duration::from_secs(24 * 60 * 60);

#[test]
fn archive_create_with_atime() {
    setup();
    TestResources::extract_in("raw/", "archive_create_with_atime/in/").unwrap();

    // Update file with newer atime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_create_with_atime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_times(FileTimes::new().set_accessed(SystemTime::now() + DURATION_24_HOURS))
        .unwrap();

    // Create archive with specified atime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_create_with_atime/create_with_atime.pna",
        "--overwrite",
        "archive_create_with_atime/in/",
        "--keep-timestamp",
        "--atime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is set correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry("archive_create_with_atime/create_with_atime.pna", |entry| {
        assert_eq!(entry.metadata().accessed(), Some(expected));
    })
    .unwrap();
}

#[test]
fn archive_create_with_clamp_atime() {
    setup();
    TestResources::extract_in("raw/", "archive_create_with_clamp_atime/in/").unwrap();

    // Update file with newer atime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_create_with_clamp_atime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_times(FileTimes::new().set_accessed(SystemTime::now() + DURATION_24_HOURS))
        .unwrap();

    // Create archive with specified atime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_create_with_clamp_atime/create_with_clamp_atime.pna",
        "--overwrite",
        "archive_create_with_clamp_atime/in/",
        "--keep-timestamp",
        "--atime",
        "2024-01-01T00:00:00Z",
        "--clamp-atime",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is clamped correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry(
        "archive_create_with_clamp_atime/create_with_clamp_atime.pna",
        |entry| {
            assert!(entry.metadata().accessed() <= Some(expected));
        },
    )
    .unwrap();
}
