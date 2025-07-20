use crate::utils::{archive::for_each_entry, setup, TestResources};
use clap::Parser;
use pna::Duration;
use portable_network_archive::{cli, command::Command};
use std::{fs, time::SystemTime};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);

#[test]
fn archive_append_with_mtime() {
    setup();
    TestResources::extract_in("raw/", "archive_append_with_mtime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append_with_mtime/append_with_mtime.pna",
        "--overwrite",
        "archive_append_with_mtime/in/",
        "--keep-timestamp",
        "--mtime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input and update their mtime
    TestResources::extract_in("store.pna", "archive_append_with_mtime/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append_with_mtime/in/").unwrap();

    let store_file = fs::File::options()
        .write(true)
        .open("archive_append_with_mtime/in/store.pna")
        .unwrap();
    store_file
        .set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    let zstd_file = fs::File::options()
        .write(true)
        .open("archive_append_with_mtime/in/zstd.pna")
        .unwrap();
    zstd_file
        .set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Append with specified mtime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--mtime",
        "2024-01-01T00:00:00Z",
        "--keep-timestamp",
        "archive_append_with_mtime/append_with_mtime.pna",
        "archive_append_with_mtime/in/store.pna",
        "archive_append_with_mtime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify mtime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry(
        "archive_append_with_mtime/append_with_mtime.pna",
        |entry| match entry.header().path().as_str() {
            "archive_append_with_mtime/in/store.pna" | "archive_append_with_mtime/in/zstd.pna" => {
                assert_eq!(entry.metadata().modified(), Some(expected))
            }
            _ => assert!(entry.metadata().modified().is_some()),
        },
    )
    .unwrap();
}

#[test]
fn archive_append_with_clamp_mtime() {
    setup();
    TestResources::extract_in("raw/", "archive_append_with_clamp_mtime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append_with_clamp_mtime/append_with_clamp_mtime.pna",
        "--overwrite",
        "archive_append_with_clamp_mtime/in/",
        "--keep-timestamp",
        "--mtime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input and update their mtime
    TestResources::extract_in("store.pna", "archive_append_with_clamp_mtime/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append_with_clamp_mtime/in/").unwrap();

    let store_file = fs::File::options()
        .write(true)
        .open("archive_append_with_clamp_mtime/in/store.pna")
        .unwrap();
    store_file
        .set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    let zstd_file = fs::File::options()
        .write(true)
        .open("archive_append_with_clamp_mtime/in/zstd.pna")
        .unwrap();
    zstd_file
        .set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Append with specified mtime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--mtime",
        "2024-01-01T00:00:00Z",
        "--clamp-mtime",
        "--keep-timestamp",
        "archive_append_with_clamp_mtime/append_with_clamp_mtime.pna",
        "archive_append_with_clamp_mtime/in/store.pna",
        "archive_append_with_clamp_mtime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify mtime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry(
        "archive_append_with_clamp_mtime/append_with_clamp_mtime.pna",
        |entry| match entry.header().path().as_str() {
            "archive_append_with_clamp_mtime/in/store.pna"
            | "archive_append_with_clamp_mtime/in/zstd.pna" => {
                assert!(entry.metadata().modified() <= Some(expected))
            }
            _ => assert!(entry.metadata().modified().is_some()),
        },
    )
    .unwrap();
}
