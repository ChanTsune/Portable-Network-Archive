#[cfg(not(target_family = "wasm"))]
use crate::utils::time::set_file_times;
use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup};
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
const CTIME_2002: Duration = Duration::seconds(1_009_843_200);
const SOURCE_MTIME: Duration = Duration::seconds(SOURCE_MTIME_SECS as i64);
const SOURCE_ATIME: Duration = Duration::seconds(SOURCE_ATIME_SECS as i64);

#[test]
fn archive_append_with_ctime() {
    setup();
    TestResources::extract_in("raw/", "archive_append_with_ctime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
        "-f",
        "archive_append_with_ctime/append_with_ctime.pna",
        "archive_append_with_ctime/in/store.pna",
        "archive_append_with_ctime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is set correctly in the archive
    let expected = Duration::seconds(1704067200);
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
        "-f",
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
        "-f",
        "archive_append_with_clamp_ctime/append_with_clamp_ctime.pna",
        "archive_append_with_clamp_ctime/in/store.pna",
        "archive_append_with_clamp_ctime/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify ctime is clamped correctly in the archive
    let expected = Duration::seconds(1704067200);
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

#[cfg(not(target_family = "wasm"))]
fn prepare_append(base: &str) -> String {
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
    added
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
/// Action: Append with `--ctime` but no preservation flag.
/// Expectation: The creation time is overridden and the other source timestamps are preserved.
#[test]
#[cfg(not(target_family = "wasm"))]
fn append_ctime_implicitly_preserves_source_timestamps() {
    setup();
    let base = "append_ctime_implicitly_preserves_source_timestamps";
    let added = prepare_append(base);

    let archive = append_archive(base, &added, &["--ctime", "2002-01-01T00:00:00Z"]);

    assert_eq!(
        entry_times(&archive, "added.txt"),
        (Some(CTIME_2002), Some(SOURCE_MTIME), Some(SOURCE_ATIME))
    );
    assert_eq!(entry_times(&archive, "seed.txt"), (None, None, None));
}
