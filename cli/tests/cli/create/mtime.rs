#[cfg(not(target_family = "wasm"))]
use crate::utils::time::set_file_times;
use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup, time::stored_time};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{
    fs,
    io::prelude::*,
    time::{SystemTime, UNIX_EPOCH},
};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);
const SOURCE_MTIME_SECS: u64 = 1_275_782_400;
const SOURCE_ATIME_SECS: u64 = 1_275_868_800;
const MTIME_2001_SECS: u64 = 978_307_200;
const SOURCE_MTIME: Duration = Duration::seconds(SOURCE_MTIME_SECS as i64);
const SOURCE_ATIME: Duration = Duration::seconds(SOURCE_ATIME_SECS as i64);
const MTIME_2001: Duration = Duration::seconds(MTIME_2001_SECS as i64);

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
        "-f",
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
    let expected = Duration::seconds(1704067200);
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
        "-f",
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
    let expected = Duration::seconds(1704067200);
    for_each_entry(
        "archive_create_with_clamp_mtime/create_with_clamp_mtime.pna",
        |entry| {
            assert!(entry.metadata().modified() <= Some(expected));
        },
    )
    .unwrap();
}

#[cfg(not(target_family = "wasm"))]
fn prepare_source(base: &str) -> (String, Option<Duration>) {
    fs::create_dir_all(base).unwrap();
    let source = format!("{base}/source.txt");
    fs::write(&source, b"content").unwrap();
    set_file_times(
        &source,
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_MTIME_SECS),
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_ATIME_SECS),
    );
    let created = fs::metadata(&source)
        .unwrap()
        .created()
        .ok()
        .map(stored_time);
    (source, created)
}

fn create_archive(base: &str, source: &str, flags: &[&str]) -> String {
    let archive = format!("{base}/archive.pna");
    let mut args = vec![
        "pna",
        "--quiet",
        "c",
        "-f",
        archive.as_str(),
        "--overwrite",
        source,
    ];
    args.extend_from_slice(flags);
    cli::Cli::try_parse_from(args).unwrap().execute().unwrap();
    archive
}

fn source_entry_times(archive: &str) -> (Option<Duration>, Option<Duration>, Option<Duration>) {
    let mut times = None;
    for_each_entry(archive, |entry| {
        if entry.header().path().as_str().ends_with("source.txt") {
            assert!(times.is_none(), "source.txt occurs more than once");
            times = Some((
                entry.metadata().created(),
                entry.metadata().modified(),
                entry.metadata().accessed(),
            ));
        }
    })
    .unwrap();
    times.expect("source.txt entry")
}

/// Precondition: A file has known modification, access, and creation times.
/// Action: Create an archive with `--mtime` but no preservation flag.
/// Expectation: The modification time is overridden and the other source timestamps are preserved.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_mtime_implicitly_preserves_source_timestamps() {
    setup();
    let base = "create_mtime_implicitly_preserves_source_timestamps";
    let (source, created) = prepare_source(base);

    let archive = create_archive(base, &source, &["--mtime", "2001-01-01T00:00:00Z"]);

    assert_eq!(
        source_entry_times(&archive),
        (created, Some(MTIME_2001), Some(SOURCE_ATIME))
    );
}

/// Precondition: A file's modification time is newer than the `--mtime` value.
/// Action: Create an archive with `--mtime` and `--clamp-mtime`.
/// Expectation: The entry's modification time is clamped to the specified value.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_mtime_clamps_newer_source_to_override() {
    setup();
    let base = "create_mtime_clamps_newer_source_to_override";
    let (source, created) = prepare_source(base);

    let archive = create_archive(
        base,
        &source,
        &["--mtime", "2001-01-01T00:00:00Z", "--clamp-mtime"],
    );

    assert_eq!(
        source_entry_times(&archive),
        (created, Some(MTIME_2001), Some(SOURCE_ATIME))
    );
}

/// Precondition: A file's modification time is older than the `--mtime` value.
/// Action: Create an archive with `--mtime` and `--clamp-mtime`.
/// Expectation: The entry keeps its source modification time.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_mtime_clamp_keeps_source_older_than_override() {
    setup();
    let base = "create_mtime_clamp_keeps_source_older_than_override";
    let (source, created) = prepare_source(base);

    let archive = create_archive(
        base,
        &source,
        &["--mtime", "2020-01-01T00:00:00Z", "--clamp-mtime"],
    );

    assert_eq!(
        source_entry_times(&archive),
        (created, Some(SOURCE_MTIME), Some(SOURCE_ATIME))
    );
}
