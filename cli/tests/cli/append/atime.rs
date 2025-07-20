use crate::utils::{archive::for_each_entry, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs::{self, FileTimes},
    time::{Duration, SystemTime},
};

const DURATION_24_HOURS: Duration = Duration::from_secs(24 * 60 * 60);

#[test]
fn archive_append_with_atime() {
    setup();
    TestResources::extract_in("raw/", "archive_append_with_atime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append_with_atime/append_with_atime.pna",
        "--overwrite",
        "archive_append_with_atime/in/",
        "--keep-timestamp",
        "--atime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input and update their atime
    TestResources::extract_in("store.pna", "archive_append_with_atime/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append_with_atime/in/").unwrap();

    let store_file = fs::File::options()
        .write(true)
        .open("archive_append_with_atime/in/store.pna")
        .unwrap();
    store_file
        .set_times(FileTimes::new().set_accessed(SystemTime::now() + DURATION_24_HOURS))
        .unwrap();

    let zstd_file = fs::File::options()
        .write(true)
        .open("archive_append_with_atime/in/zstd.pna")
        .unwrap();
    zstd_file
        .set_times(FileTimes::new().set_accessed(SystemTime::now() + DURATION_24_HOURS))
        .unwrap();

    // Append with specified atime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--atime",
        "2024-01-01T00:00:00Z",
        "--keep-timestamp",
        "archive_append_with_atime/append_with_atime.pna",
        "archive_append_with_atime/in/store.pna",
        "archive_append_with_atime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is set correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry(
        "archive_append_with_atime/append_with_atime.pna",
        |entry| match entry.header().path().as_str() {
            "archive_append_with_atime/in/store.pna" | "archive_append_with_atime/in/zstd.pna" => {
                assert_eq!(entry.metadata().accessed(), Some(expected))
            }
            _ => assert!(entry.metadata().accessed().is_some()),
        },
    )
    .unwrap();
}

#[test]
fn archive_append_with_clamp_atime() {
    setup();
    TestResources::extract_in("raw/", "archive_append_with_clamp_atime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append_with_clamp_atime/append_with_clamp_atime.pna",
        "--overwrite",
        "archive_append_with_clamp_atime/in/",
        "--keep-timestamp",
        "--atime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input and update their atime
    TestResources::extract_in("store.pna", "archive_append_with_clamp_atime/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append_with_clamp_atime/in/").unwrap();

    let store_file = fs::File::options()
        .write(true)
        .open("archive_append_with_clamp_atime/in/store.pna")
        .unwrap();
    store_file
        .set_times(FileTimes::new().set_accessed(SystemTime::now() + DURATION_24_HOURS))
        .unwrap();

    let zstd_file = fs::File::options()
        .write(true)
        .open("archive_append_with_clamp_atime/in/zstd.pna")
        .unwrap();
    zstd_file
        .set_times(FileTimes::new().set_accessed(SystemTime::now() + DURATION_24_HOURS))
        .unwrap();

    // Append with specified atime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--atime",
        "2024-01-01T00:00:00Z",
        "--clamp-atime",
        "--keep-timestamp",
        "archive_append_with_clamp_atime/append_with_clamp_atime.pna",
        "archive_append_with_clamp_atime/in/store.pna",
        "archive_append_with_clamp_atime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is clamped correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry(
        "archive_append_with_clamp_atime/append_with_clamp_atime.pna",
        |entry| match entry.header().path().as_str() {
            "archive_append_with_clamp_atime/in/store.pna"
            | "archive_append_with_clamp_atime/in/zstd.pna" => {
                assert!(entry.metadata().accessed() <= Some(expected))
            }
            _ => assert!(entry.metadata().accessed().is_some()),
        },
    )
    .unwrap();
}
