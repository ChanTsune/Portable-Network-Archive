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
fn archive_create_with_ctime() {
    setup();
    TestResources::extract_in("raw/", "archive_create_with_ctime/in/").unwrap();

    // Update file with newer ctime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_create_with_ctime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Create archive with specified ctime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_create_with_ctime/create_with_ctime.pna",
        "--overwrite",
        "archive_create_with_ctime/in/",
        "--keep-timestamp",
        "--ctime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is set correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry("archive_create_with_ctime/create_with_ctime.pna", |entry| {
        assert_eq!(entry.metadata().created(), Some(expected));
    })
    .unwrap();
}

#[test]
fn archive_create_with_clamp_ctime() {
    setup();
    TestResources::extract_in("raw/", "archive_create_with_clamp_ctime/in/").unwrap();

    // Update file with newer ctime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_create_with_clamp_ctime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Create archive with specified ctime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_create_with_clamp_ctime/create_with_clamp_ctime.pna",
        "--overwrite",
        "archive_create_with_clamp_ctime/in/",
        "--keep-timestamp",
        "--ctime",
        "2024-01-01T00:00:00Z",
        "--clamp-ctime",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is clamped correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry(
        "archive_create_with_clamp_ctime/create_with_clamp_ctime.pna",
        |entry| {
            assert!(entry.metadata().created() <= Some(expected));
        },
    )
    .unwrap();
}
