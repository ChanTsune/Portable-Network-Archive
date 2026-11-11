#[cfg(not(target_family = "wasm"))]
use crate::utils::time::set_file_times;
use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup, time::stored_time};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);
const SOURCE_MTIME_SECS: u64 = 1_275_782_400;
const SOURCE_ATIME_SECS: u64 = 1_275_868_800;
const MTIME_2001: Duration = Duration::seconds(978_307_200);
const SOURCE_MTIME: Duration = Duration::seconds(SOURCE_MTIME_SECS as i64);
const SOURCE_ATIME: Duration = Duration::seconds(SOURCE_ATIME_SECS as i64);

#[test]
fn archive_append_with_mtime() {
    setup();
    TestResources::extract_in("raw/", "archive_append_with_mtime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
        "-f",
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
        "-f",
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
        "-f",
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
/// Action: Append with `--mtime` but no preservation flag.
/// Expectation: The modification time is overridden and the other source timestamps are preserved.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_mtime_implicitly_preserves_source_timestamps() {
    setup();
    let base = "append_mtime_implicitly_preserves_source_timestamps";
    let (added, created) = prepare_append(base);

    let archive = append_archive(base, &added, &["--mtime", "2001-01-01T00:00:00Z"]);

    assert_eq!(
        entry_times(&archive, "added.txt"),
        (created, Some(MTIME_2001), Some(SOURCE_ATIME))
    );
    assert_eq!(entry_times(&archive, "seed.txt"), (None, None, None));
}

/// Precondition: A file's modification time is newer than the `--mtime` value.
/// Action: Append with `--mtime` and `--clamp-mtime`.
/// Expectation: The appended entry's modification time is clamped to the specified value.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_mtime_clamps_newer_source_to_override() {
    setup();
    let base = "append_mtime_clamps_newer_source_to_override";
    let (added, created) = prepare_append(base);

    let archive = append_archive(
        base,
        &added,
        &["--mtime", "2001-01-01T00:00:00Z", "--clamp-mtime"],
    );

    assert_eq!(
        entry_times(&archive, "added.txt"),
        (created, Some(MTIME_2001), Some(SOURCE_ATIME))
    );
}

/// Precondition: A file's modification time is older than the `--mtime` value.
/// Action: Append with `--mtime` and `--clamp-mtime`.
/// Expectation: The appended entry keeps its source modification time.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_mtime_clamp_keeps_source_older_than_override() {
    setup();
    let base = "append_mtime_clamp_keeps_source_older_than_override";
    let (added, created) = prepare_append(base);

    let archive = append_archive(
        base,
        &added,
        &["--mtime", "2020-01-01T00:00:00Z", "--clamp-mtime"],
    );

    assert_eq!(
        entry_times(&archive, "added.txt"),
        (created, Some(SOURCE_MTIME), Some(SOURCE_ATIME))
    );
}
