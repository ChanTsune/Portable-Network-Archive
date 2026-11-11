#[cfg(not(target_family = "wasm"))]
use crate::utils::time::set_file_times;
use crate::utils::{archive::for_each_entry, setup, time::stored_time};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{fs, time::UNIX_EPOCH};

const SOURCE_MTIME_SECS: u64 = 1_275_782_400;
const SOURCE_ATIME_SECS: u64 = 1_275_868_800;
const SOURCE_MTIME: Duration = Duration::seconds(SOURCE_MTIME_SECS as i64);
const SOURCE_ATIME: Duration = Duration::seconds(SOURCE_ATIME_SECS as i64);

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

/// Precondition: A file has known timestamps.
/// Action: Create an archive without a timestamp preservation flag.
/// Expectation: The entry stores no timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_without_preserve_flag_omits_source_timestamps() {
    setup();
    let base = "create_without_preserve_flag_omits_source_timestamps";
    let (source, _created) = prepare_source(base);

    let archive = create_archive(base, &source, &[]);

    assert_eq!(source_entry_times(&archive), (None, None, None));
}

/// Precondition: A file has known timestamps.
/// Action: Create an archive with `--preserve-timestamps`.
/// Expectation: The entry stores the source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_with_preserve_timestamps_stores_source_timestamps() {
    setup();
    let base = "create_with_preserve_timestamps_stores_source_timestamps";
    let (source, created) = prepare_source(base);

    let archive = create_archive(base, &source, &["--preserve-timestamps"]);

    assert_eq!(
        source_entry_times(&archive),
        (created, Some(SOURCE_MTIME), Some(SOURCE_ATIME))
    );
}

/// Precondition: A file has known timestamps.
/// Action: Create an archive with the `--keep-timestamp` alias.
/// Expectation: The entry stores the source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_with_keep_timestamp_alias_stores_source_timestamps() {
    setup();
    let base = "create_with_keep_timestamp_alias_stores_source_timestamps";
    let (source, created) = prepare_source(base);

    let archive = create_archive(base, &source, &["--keep-timestamp"]);

    assert_eq!(
        source_entry_times(&archive),
        (created, Some(SOURCE_MTIME), Some(SOURCE_ATIME))
    );
}

/// Precondition: A file has known timestamps.
/// Action: Create an archive with `--no-preserve-timestamps`.
/// Expectation: The entry stores no timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_with_no_preserve_timestamps_omits_source_timestamps() {
    setup();
    let base = "create_with_no_preserve_timestamps_omits_source_timestamps";
    let (source, _created) = prepare_source(base);

    let archive = create_archive(base, &source, &["--no-preserve-timestamps"]);

    assert_eq!(source_entry_times(&archive), (None, None, None));
}

/// Precondition: A file has known timestamps.
/// Action: Create an archive with the `--no-keep-timestamp` alias.
/// Expectation: The entry stores no timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_with_no_keep_timestamp_alias_omits_source_timestamps() {
    setup();
    let base = "create_with_no_keep_timestamp_alias_omits_source_timestamps";
    let (source, _created) = prepare_source(base);

    let archive = create_archive(base, &source, &["--no-keep-timestamp"]);

    assert_eq!(source_entry_times(&archive), (None, None, None));
}

/// Precondition: A file has known timestamps.
/// Action: Use a negative preservation flag with each time override.
/// Expectation: The negative flag takes precedence over the override.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_negative_timestamp_flag_overrides_time_options() {
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
        let base = format!("create_negative_timestamp_flag_overrides_time_options_{index}");
        let (source, _) = prepare_source(&base);
        let archive = create_archive(&base, &source, flags);
        assert_eq!(source_entry_times(&archive), (None, None, None));
    }
}

/// Precondition: Timestamp preservation flags have opposite meanings.
/// Action: Supply both preservation flags to each archive operation.
/// Expectation: The command rejects the conflicting flags.
#[test]
fn timestamp_preservation_flags_conflict() {
    for args in [
        vec![
            "pna",
            "create",
            "-f",
            "archive.pna",
            "--preserve-timestamps",
            "--no-preserve-timestamps",
            "file",
        ],
        vec![
            "pna",
            "append",
            "-f",
            "archive.pna",
            "--preserve-timestamps",
            "--no-preserve-timestamps",
            "file",
        ],
        vec![
            "pna",
            "update",
            "-f",
            "archive.pna",
            "--preserve-timestamps",
            "--no-preserve-timestamps",
            "file",
        ],
        vec![
            "pna",
            "extract",
            "-f",
            "archive.pna",
            "--preserve-timestamps",
            "--no-preserve-timestamps",
        ],
        vec![
            "pna",
            "compat",
            "bsdtar",
            "-c",
            "-f",
            "archive.pna",
            "--preserve-timestamps",
            "--no-preserve-timestamps",
            "file",
        ],
    ] {
        assert_eq!(
            cli::Cli::try_parse_from(args).err().unwrap().kind(),
            clap::error::ErrorKind::ArgumentConflict
        );
    }
}
