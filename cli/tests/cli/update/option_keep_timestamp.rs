#[cfg(not(target_family = "wasm"))]
use crate::utils::time::set_file_times;
use crate::utils::{archive::for_each_entry, setup, time::stored_time};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{fs, time::UNIX_EPOCH};

const SOURCE_MTIME_SECS: u64 = 1_275_782_400;
const UPDATED_MTIME_SECS: u64 = 1_600_000_000;
const SOURCE_ATIME_SECS: u64 = 1_275_868_800;
const UPDATED_ATIME_SECS: u64 = 1_600_086_400;
const SOURCE_MTIME: Duration = Duration::seconds(SOURCE_MTIME_SECS as i64);
const UPDATED_MTIME: Duration = Duration::seconds(UPDATED_MTIME_SECS as i64);
const SOURCE_ATIME: Duration = Duration::seconds(SOURCE_ATIME_SECS as i64);
const UPDATED_ATIME: Duration = Duration::seconds(UPDATED_ATIME_SECS as i64);

struct Prepared {
    source: String,
    original_created: Option<Duration>,
    revised_created: Option<Duration>,
}

#[cfg(not(target_family = "wasm"))]
fn prepare_update(base: &str) -> Prepared {
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
    Prepared {
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

fn assert_original_preserved(
    times: &[(Option<Duration>, Option<Duration>, Option<Duration>)],
    created: Option<Duration>,
) {
    assert_eq!(times[0], (created, Some(SOURCE_MTIME), Some(SOURCE_ATIME)));
}

/// Precondition: An archive stores a file with source timestamps.
/// Action: Update the file without a timestamp preservation flag.
/// Expectation: The revised entry stores no timestamps while the original entry keeps its source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn update_without_preserve_flag_omits_revised_timestamps() {
    setup();
    let base = "update_without_preserve_flag_omits_revised_timestamps";
    let prepared = prepare_update(base);

    let archive = update_archive(base, &prepared.source, &[]);

    let times = entry_times(&archive, "source.txt");
    assert_eq!(times.len(), 2);
    assert_original_preserved(&times, prepared.original_created);
    assert_eq!(times[1], (None, None, None));
}

/// Precondition: An archive stores a file with source timestamps.
/// Action: Update the file with `--preserve-timestamps`.
/// Expectation: The revised entry stores the updated source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn update_with_preserve_timestamps_stores_revised_timestamps() {
    setup();
    let base = "update_with_preserve_timestamps_stores_revised_timestamps";
    let prepared = prepare_update(base);

    let archive = update_archive(base, &prepared.source, &["--preserve-timestamps"]);

    let times = entry_times(&archive, "source.txt");
    assert_eq!(times.len(), 2);
    assert_original_preserved(&times, prepared.original_created);
    assert_eq!(
        times[1],
        (
            prepared.revised_created,
            Some(UPDATED_MTIME),
            Some(UPDATED_ATIME)
        )
    );
}

/// Precondition: An archive stores a file with source timestamps.
/// Action: Update the file with the `--keep-timestamp` alias.
/// Expectation: The revised entry stores the updated source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn update_with_keep_timestamp_alias_stores_revised_timestamps() {
    setup();
    let base = "update_with_keep_timestamp_alias_stores_revised_timestamps";
    let prepared = prepare_update(base);

    let archive = update_archive(base, &prepared.source, &["--keep-timestamp"]);

    let times = entry_times(&archive, "source.txt");
    assert_eq!(times.len(), 2);
    assert_original_preserved(&times, prepared.original_created);
    assert_eq!(
        times[1],
        (
            prepared.revised_created,
            Some(UPDATED_MTIME),
            Some(UPDATED_ATIME)
        )
    );
}

/// Precondition: An archive stores a file with source timestamps.
/// Action: Update the file with `--no-preserve-timestamps`.
/// Expectation: The revised entry stores no timestamps while the original entry keeps its source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn update_with_no_preserve_timestamps_omits_revised_timestamps() {
    setup();
    let base = "update_with_no_preserve_timestamps_omits_revised_timestamps";
    let prepared = prepare_update(base);

    let archive = update_archive(base, &prepared.source, &["--no-preserve-timestamps"]);

    let times = entry_times(&archive, "source.txt");
    assert_eq!(times.len(), 2);
    assert_original_preserved(&times, prepared.original_created);
    assert_eq!(times[1], (None, None, None));
}

/// Precondition: An archive stores a file with source timestamps.
/// Action: Update the file with the `--no-keep-timestamp` alias.
/// Expectation: The revised entry stores no timestamps while the original entry keeps its source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn update_with_no_keep_timestamp_alias_omits_revised_timestamps() {
    setup();
    let base = "update_with_no_keep_timestamp_alias_omits_revised_timestamps";
    let prepared = prepare_update(base);

    let archive = update_archive(base, &prepared.source, &["--no-keep-timestamp"]);

    let times = entry_times(&archive, "source.txt");
    assert_eq!(times.len(), 2);
    assert_original_preserved(&times, prepared.original_created);
    assert_eq!(times[1], (None, None, None));
}

/// Precondition: An archive stores a file without timestamp metadata.
/// Action: Update the modified file with `--preserve-timestamps`.
/// Expectation: The original entry keeps no timestamps and the revised entry gains the updated source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn update_with_preserve_timestamps_gains_timestamps_for_untimestamped_archive() {
    setup();
    let base = "update_with_preserve_timestamps_gains_timestamps_for_untimestamped_archive";
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(base).unwrap();
    let source = format!("{base}/source.txt");
    fs::write(&source, b"v1").unwrap();
    set_file_times(
        &source,
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_MTIME_SECS),
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_ATIME_SECS),
    );
    let archive = format!("{base}/archive.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        archive.as_str(),
        "--overwrite",
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

    let archive = update_archive(base, &source, &["--preserve-timestamps"]);

    let times = entry_times(&archive, "source.txt");
    assert_eq!(times.len(), 2);
    assert_eq!(times[0], (None, None, None));
    assert_eq!(
        times[1],
        (revised_created, Some(UPDATED_MTIME), Some(UPDATED_ATIME))
    );
}

/// Precondition: A file has known timestamps.
/// Action: Use a negative preservation flag with each time override.
/// Expectation: The negative flag takes precedence over the override.
#[test]
#[cfg(not(target_family = "wasm"))]
fn update_negative_timestamp_flag_overrides_time_options() {
    setup();
    for (index, flags) in [
        [
            "--no-preserve-timestamps",
            "--mtime",
            "2001-01-01T00:00:00Z",
        ],
        [
            "--no-preserve-timestamps",
            "--ctime",
            "2001-01-01T00:00:00Z",
        ],
        [
            "--no-preserve-timestamps",
            "--atime",
            "2001-01-01T00:00:00Z",
        ],
    ]
    .iter()
    .enumerate()
    {
        let base = format!("update_negative_timestamp_flag_overrides_time_options_{index}");
        let prepared = prepare_update(&base);
        let archive = update_archive(&base, &prepared.source, flags);
        let times = entry_times(&archive, "source.txt");
        assert_eq!(times.len(), 2);
        assert_original_preserved(&times, prepared.original_created);
        assert_eq!(times[1], (None, None, None));
    }
}
