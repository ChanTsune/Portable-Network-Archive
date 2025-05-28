use crate::utils::{archive::for_each_entry, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs,
    time::{Duration, SystemTime},
};

const DURATION_24_HOURS: Duration = Duration::from_secs(24 * 60 * 60);

#[test]
fn archive_append_with_ctime() {
    setup();
    TestResources::extract_in("raw/", "archive_append_with_ctime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append_with_ctime/append_with_ctime.pna",
        "--overwrite",
        "archive_append_with_ctime/in/",
        "--keep-timestamp",
        "--ctime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input and update their ctime
    TestResources::extract_in("store.pna", "archive_append_with_ctime/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append_with_ctime/in/").unwrap();

    let store_file = fs::File::options()
        .write(true)
        .open("archive_append_with_ctime/in/store.pna")
        .unwrap();
    store_file
        .set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    let zstd_file = fs::File::options()
        .write(true)
        .open("archive_append_with_ctime/in/zstd.pna")
        .unwrap();
    zstd_file
        .set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Append with specified ctime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--ctime",
        "2024-01-01T00:00:00Z",
        "--keep-timestamp",
        "archive_append_with_ctime/append_with_ctime.pna",
        "archive_append_with_ctime/in/store.pna",
        "archive_append_with_ctime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is set correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry(
        "archive_append_with_ctime/append_with_ctime.pna",
        |entry| match entry.header().path().as_str() {
            "archive_append_with_ctime/in/store.pna" | "archive_append_with_ctime/in/zstd.pna" => {
                assert_eq!(entry.metadata().created(), Some(expected))
            }
            _ => assert!(entry.metadata().created().is_some()),
        },
    )
    .unwrap();
}

#[test]
fn archive_append_with_clamp_ctime() {
    setup();
    TestResources::extract_in("raw/", "archive_append_with_clamp_ctime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append_with_clamp_ctime/append_with_clamp_ctime.pna",
        "--overwrite",
        "archive_append_with_clamp_ctime/in/",
        "--keep-timestamp",
        "--ctime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input and update their ctime
    TestResources::extract_in("store.pna", "archive_append_with_clamp_ctime/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append_with_clamp_ctime/in/").unwrap();

    let store_file = fs::File::options()
        .write(true)
        .open("archive_append_with_clamp_ctime/in/store.pna")
        .unwrap();
    store_file
        .set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    let zstd_file = fs::File::options()
        .write(true)
        .open("archive_append_with_clamp_ctime/in/zstd.pna")
        .unwrap();
    zstd_file
        .set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Append with specified ctime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--ctime",
        "2024-01-01T00:00:00Z",
        "--clamp-ctime",
        "--keep-timestamp",
        "archive_append_with_clamp_ctime/append_with_clamp_ctime.pna",
        "archive_append_with_clamp_ctime/in/store.pna",
        "archive_append_with_clamp_ctime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is clamped correctly in the archive
    let expected = Duration::from_secs(1704067200);
    for_each_entry(
        "archive_append_with_clamp_ctime/append_with_clamp_ctime.pna",
        |entry| match entry.header().path().as_str() {
            "archive_append_with_clamp_ctime/in/store.pna"
            | "archive_append_with_clamp_ctime/in/zstd.pna" => {
                assert!(entry.metadata().created() <= Some(expected))
            }
            _ => assert!(entry.metadata().created().is_some()),
        },
    )
    .unwrap();
}
