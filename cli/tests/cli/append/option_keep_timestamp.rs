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
fn prepare_append(base: &str) -> (String, Option<Duration>) {
    fs::create_dir_all(base).unwrap();
    let seed = format!("{base}/seed.txt");
    fs::write(&seed, b"seed").unwrap();
    let archive = format!("{base}/archive.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        archive.as_str(),
        "--overwrite",
        seed.as_str(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let added = format!("{base}/added.txt");
    fs::write(&added, b"added").unwrap();
    set_file_times(
        &added,
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_MTIME_SECS),
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_ATIME_SECS),
    );
    let created = fs::metadata(&added)
        .unwrap()
        .created()
        .ok()
        .map(stored_time);
    (added, created)
}

fn append_archive(base: &str, added: &str, flags: &[&str]) -> String {
    let archive = format!("{base}/archive.pna");
    let mut args = vec!["pna", "--quiet", "append", "-f", archive.as_str(), added];
    args.extend_from_slice(flags);
    cli::Cli::try_parse_from(args).unwrap().execute().unwrap();
    archive
}

fn entry_times(
    archive: &str,
    name: &str,
) -> (Option<Duration>, Option<Duration>, Option<Duration>) {
    let mut times = None;
    for_each_entry(archive, |entry| {
        if entry.header().path().as_str().ends_with(name) {
            assert!(times.is_none(), "{name} occurs more than once");
            times = Some((
                entry.metadata().created(),
                entry.metadata().modified(),
                entry.metadata().accessed(),
            ));
        }
    })
    .unwrap();
    times.unwrap_or_else(|| panic!("{name} entry"))
}

/// Precondition: A file is appended to an archive without timestamp metadata.
/// Action: Append without a timestamp preservation flag.
/// Expectation: The appended entry stores no timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_without_preserve_flag_omits_source_timestamps() {
    setup();
    let base = "append_without_preserve_flag_omits_source_timestamps";
    let (added, _created) = prepare_append(base);

    let archive = append_archive(base, &added, &[]);

    assert_eq!(entry_times(&archive, "added.txt"), (None, None, None));
    assert_eq!(entry_times(&archive, "seed.txt"), (None, None, None));
}

/// Precondition: A file is appended to an archive without timestamp metadata.
/// Action: Append with `--preserve-timestamps`.
/// Expectation: The appended entry stores the source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_with_preserve_timestamps_stores_source_timestamps() {
    setup();
    let base = "append_with_preserve_timestamps_stores_source_timestamps";
    let (added, created) = prepare_append(base);

    let archive = append_archive(base, &added, &["--preserve-timestamps"]);

    assert_eq!(
        entry_times(&archive, "added.txt"),
        (created, Some(SOURCE_MTIME), Some(SOURCE_ATIME))
    );
    assert_eq!(entry_times(&archive, "seed.txt"), (None, None, None));
}

/// Precondition: A file is appended to an archive without timestamp metadata.
/// Action: Append with the `--keep-timestamp` alias.
/// Expectation: The appended entry stores the source timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_with_keep_timestamp_alias_stores_source_timestamps() {
    setup();
    let base = "append_with_keep_timestamp_alias_stores_source_timestamps";
    let (added, created) = prepare_append(base);

    let archive = append_archive(base, &added, &["--keep-timestamp"]);

    assert_eq!(
        entry_times(&archive, "added.txt"),
        (created, Some(SOURCE_MTIME), Some(SOURCE_ATIME))
    );
}

/// Precondition: A file is appended to an archive without timestamp metadata.
/// Action: Append with `--no-preserve-timestamps`.
/// Expectation: The appended entry stores no timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_with_no_preserve_timestamps_omits_source_timestamps() {
    setup();
    let base = "append_with_no_preserve_timestamps_omits_source_timestamps";
    let (added, _created) = prepare_append(base);

    let archive = append_archive(base, &added, &["--no-preserve-timestamps"]);

    assert_eq!(entry_times(&archive, "added.txt"), (None, None, None));
}

/// Precondition: A file is appended to an archive without timestamp metadata.
/// Action: Append with the `--no-keep-timestamp` alias.
/// Expectation: The appended entry stores no timestamps.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_with_no_keep_timestamp_alias_omits_source_timestamps() {
    setup();
    let base = "append_with_no_keep_timestamp_alias_omits_source_timestamps";
    let (added, _created) = prepare_append(base);

    let archive = append_archive(base, &added, &["--no-keep-timestamp"]);

    assert_eq!(entry_times(&archive, "added.txt"), (None, None, None));
}

/// Precondition: A file has known timestamps.
/// Action: Use a negative preservation flag with each time override.
/// Expectation: The negative flag takes precedence over the override.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_negative_timestamp_flag_overrides_time_options() {
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
        let base = format!("append_negative_timestamp_flag_overrides_time_options_{index}");
        let (added, _) = prepare_append(&base);
        let archive = append_archive(&base, &added, flags);
        assert_eq!(entry_times(&archive, "added.txt"), (None, None, None));
        assert_eq!(entry_times(&archive, "seed.txt"), (None, None, None));
    }
}
