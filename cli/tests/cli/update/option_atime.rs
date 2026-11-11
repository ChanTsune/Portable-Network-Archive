#[cfg(not(target_family = "wasm"))]
use crate::utils::time::set_file_times;
use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup, time::stored_time};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{
    fs::{self, FileTimes},
    io::prelude::*,
    time::{SystemTime, UNIX_EPOCH},
};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);
const SOURCE_MTIME_SECS: u64 = 1_275_782_400;
const UPDATED_MTIME_SECS: u64 = 1_600_000_000;
const SOURCE_ATIME_SECS: u64 = 1_275_868_800;
const UPDATED_ATIME_SECS: u64 = 1_600_086_400;
const SOURCE_MTIME: Duration = Duration::seconds(SOURCE_MTIME_SECS as i64);
const UPDATED_MTIME: Duration = Duration::seconds(UPDATED_MTIME_SECS as i64);
const SOURCE_ATIME: Duration = Duration::seconds(SOURCE_ATIME_SECS as i64);
const ATIME_2003: Duration = Duration::seconds(1_041_379_200);

/// Precondition: An archive contains files.
/// Action: Modify a file, run `pna experimental update --sync` with `--atime`.
/// Expectation: All entries in the archive have the specified atime.
///   `--sync` forces realignment of the archive to the disk state, replacing
///   pre-existing entries; without `--sync` the original copies would remain.
#[test]
fn update_with_atime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_with_atime");
    TestResources::extract_in("raw/", "update_with_atime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_with_atime/archive.pna",
        "--overwrite",
        "update_with_atime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer atime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_atime/in/raw/text.txt")
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
        "update_with_atime/archive.pna",
        "update_with_atime/in/",
        "--keep-timestamp",
        "--sync",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("update_with_atime/archive.pna", |entry| {
        assert_eq!(entry.metadata().accessed(), Some(expected));
    })
    .unwrap();
}

/// Precondition: An archive contains files.
/// Action: Modify a file, run `pna experimental update` with `--atime` and `--clamp-atime`.
/// Expectation: All entries in the archive have atime clamped to the specified value.
#[test]
fn update_with_clamp_atime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_with_clamp_atime");
    TestResources::extract_in("raw/", "update_with_clamp_atime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_with_clamp_atime/archive.pna",
        "--overwrite",
        "update_with_clamp_atime/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file with newer atime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_clamp_atime/in/raw/text.txt")
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
        "update_with_clamp_atime/archive.pna",
        "update_with_clamp_atime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("update_with_clamp_atime/archive.pna", |entry| {
        assert!(entry.metadata().accessed() <= Some(expected));
    })
    .unwrap();
}

struct PreparedUpdate {
    source: String,
    original_created: Option<Duration>,
    revised_created: Option<Duration>,
}

#[cfg(not(target_family = "wasm"))]
fn prepare_update(base: &str) -> PreparedUpdate {
    fs::create_dir_all(base).unwrap();
    let source = format!("{base}/source.txt");
    fs::write(&source, b"v1").unwrap();
    set_file_times(
        &source,
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_MTIME_SECS),
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_ATIME_SECS),
    );
    let original_created = fs::metadata(&source)
        .unwrap()
        .created()
        .ok()
        .map(stored_time);
    let archive = format!("{base}/archive.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        archive.as_str(),
        "--overwrite",
        "--preserve-timestamps",
        source.as_str(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::write(&source, b"v2 with different content").unwrap();
    set_file_times(
        &source,
        UNIX_EPOCH + std::time::Duration::from_secs(UPDATED_MTIME_SECS),
        UNIX_EPOCH + std::time::Duration::from_secs(UPDATED_ATIME_SECS),
    );
    let revised_created = fs::metadata(&source)
        .unwrap()
        .created()
        .ok()
        .map(stored_time);
    PreparedUpdate {
        source,
        original_created,
        revised_created,
    }
}

fn update_archive(base: &str, source: &str, flags: &[&str]) -> String {
    let archive = format!("{base}/archive.pna");
    let mut args = vec!["pna", "--quiet", "update", "-f", archive.as_str(), source];
    args.extend_from_slice(flags);
    cli::Cli::try_parse_from(args).unwrap().execute().unwrap();
    archive
}

fn entry_times(
    archive: &str,
    name: &str,
) -> Vec<(Option<Duration>, Option<Duration>, Option<Duration>)> {
    let mut times = Vec::new();
    for_each_entry(archive, |entry| {
        if entry.header().path().as_str().ends_with(name) {
            times.push((
                entry.metadata().created(),
                entry.metadata().modified(),
                entry.metadata().accessed(),
            ));
        }
    })
    .unwrap();
    times
}

/// Precondition: An archive stores a file with source timestamps.
/// Action: Update the file with `--atime` but no preservation flag.
/// Expectation: The revised entry overrides atime and implicitly preserves the other source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn update_with_atime_implicitly_preserves_source_timestamps() {
    setup();
    let base = "update_with_atime_implicitly_preserves_source_timestamps";
    let prepared = prepare_update(base);

    let archive = update_archive(base, &prepared.source, &["--atime", "2003-01-01T00:00:00Z"]);

    let times = entry_times(&archive, "source.txt");
    assert_eq!(times.len(), 2);
    assert_eq!(
        times[0],
        (
            prepared.original_created,
            Some(SOURCE_MTIME),
            Some(SOURCE_ATIME)
        )
    );
    assert_eq!(
        times[1],
        (
            prepared.revised_created,
            Some(UPDATED_MTIME),
            Some(ATIME_2003)
        )
    );
}
