#[cfg(not(target_family = "wasm"))]
use crate::utils::time::set_file_times;
use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup, time::stored_time};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{
    fs::{self, FileTimes},
    time::{SystemTime, UNIX_EPOCH},
};

const DURATION_24_HOURS: Duration = Duration::seconds(24 * 60 * 60);
const SOURCE_MTIME_SECS: u64 = 1_275_782_400;
const SOURCE_ATIME_SECS: u64 = 1_275_868_800;
const ATIME_2003: Duration = Duration::seconds(1_041_379_200);
const SOURCE_MTIME: Duration = Duration::seconds(SOURCE_MTIME_SECS as i64);

#[test]
fn archive_append_with_atime() {
    setup();
    TestResources::extract_in("raw/", "archive_append_with_atime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
        "-f",
        "archive_append_with_atime/append_with_atime.pna",
        "archive_append_with_atime/in/store.pna",
        "archive_append_with_atime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
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
        "-f",
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
        "-f",
        "archive_append_with_clamp_atime/append_with_clamp_atime.pna",
        "archive_append_with_clamp_atime/in/store.pna",
        "archive_append_with_clamp_atime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify atime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
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
/// Action: Append with `--atime` but no preservation flag.
/// Expectation: The access time is overridden and the other source timestamps are preserved.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_atime_implicitly_preserves_source_timestamps() {
    setup();
    let base = "append_atime_implicitly_preserves_source_timestamps";
    let (added, created) = prepare_append(base);

    let archive = append_archive(base, &added, &["--atime", "2003-01-01T00:00:00Z"]);

    assert_eq!(
        entry_times(&archive, "added.txt"),
        (created, Some(SOURCE_MTIME), Some(ATIME_2003))
    );
    assert_eq!(entry_times(&archive, "seed.txt"), (None, None, None));
}
