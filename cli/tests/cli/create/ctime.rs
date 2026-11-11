#[cfg(not(target_family = "wasm"))]
use crate::utils::time::set_file_times;
use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup};
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
const CTIME_2002: Duration = Duration::seconds(1_009_843_200);
const SOURCE_MTIME: Duration = Duration::seconds(SOURCE_MTIME_SECS as i64);
const SOURCE_ATIME: Duration = Duration::seconds(SOURCE_ATIME_SECS as i64);

#[test]
fn archive_create_with_ctime() {
    setup();
    TestResources::extract_in("raw/", "archive_create_with_ctime/in/").unwrap();

    // Update file with newer ctime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_create_with_ctime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Create archive with specified ctime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "archive_create_with_ctime/create_with_ctime.pna",
        "--overwrite",
        "archive_create_with_ctime/in/",
        "--keep-timestamp",
        "--ctime",
        "2024-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry("archive_create_with_ctime/create_with_ctime.pna", |entry| {
        assert_eq!(entry.metadata().created(), Some(expected));
    })
    .unwrap();
}

#[test]
fn archive_create_with_clamp_ctime() {
    setup();
    TestResources::extract_in("raw/", "archive_create_with_clamp_ctime/in/").unwrap();

    // Update file with newer ctime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_create_with_clamp_ctime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Create archive with specified ctime and clamp
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "archive_create_with_clamp_ctime/create_with_clamp_ctime.pna",
        "--overwrite",
        "archive_create_with_clamp_ctime/in/",
        "--keep-timestamp",
        "--ctime",
        "2024-01-01T00:00:00Z",
        "--clamp-ctime",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
    for_each_entry(
        "archive_create_with_clamp_ctime/create_with_clamp_ctime.pna",
        |entry| {
            assert!(entry.metadata().created() <= Some(expected));
        },
    )
    .unwrap();
}

#[cfg(not(target_family = "wasm"))]
fn prepare_source(base: &str) -> String {
    fs::create_dir_all(base).unwrap();
    let source = format!("{base}/source.txt");
    fs::write(&source, b"content").unwrap();
    set_file_times(
        &source,
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_MTIME_SECS),
        UNIX_EPOCH + std::time::Duration::from_secs(SOURCE_ATIME_SECS),
    );
    source
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

/// Precondition: A file has known modification and access times.
/// Action: Create an archive with `--ctime` but no preservation flag.
/// Expectation: The creation time is overridden and the other source timestamps are preserved.
#[test]
#[cfg(not(target_family = "wasm"))]
fn create_ctime_implicitly_preserves_source_timestamps() {
    setup();
    let base = "create_ctime_implicitly_preserves_source_timestamps";
    let source = prepare_source(base);

    let archive = create_archive(base, &source, &["--ctime", "2002-01-01T00:00:00Z"]);

    assert_eq!(
        source_entry_times(&archive),
        (Some(CTIME_2002), Some(SOURCE_MTIME), Some(SOURCE_ATIME))
    );
}
