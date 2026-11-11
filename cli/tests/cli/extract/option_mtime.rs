use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

fn extract_mtime(path: impl AsRef<Path>) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

fn assert_same_second(actual: SystemTime, expected: SystemTime, label: &str) {
    let actual = actual
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expected = expected
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(actual, expected, "{label} mismatch");
}

/// Precondition: Archive contains a file with mtime 2020-01-15.
/// Action: Extract with `--mtime 2024-06-01` (override mode).
/// Expectation: Extracted file has mtime 2024-06-01, with the archive timestamp ignored.
#[test]
fn extract_with_mtime_override() {
    setup();

    fs::create_dir_all("extract_mtime_override/in").unwrap();
    fs::write("extract_mtime_override/in/file.txt", "content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "extract_mtime_override/archive.pna",
        "--overwrite",
        "extract_mtime_override/in/file.txt",
        "--keep-timestamp",
        "--mtime",
        "2020-01-15T12:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_mtime_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_mtime_override/out",
        "--mtime",
        "2024-06-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mtime = extract_mtime("extract_mtime_override/out/extract_mtime_override/in/file.txt");
    let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(1717200000); // 2024-06-01
    assert_same_second(mtime, expected, "mtime override");
}

/// Precondition: Archive contains a file with mtime 2020-01-15.
/// Action: Extract with `--mtime 2025-01-01 --clamp-mtime`.
/// Expectation: Extracted file has mtime 2020-01-15 (archive time is older, so it's preserved).
#[test]
fn extract_with_clamp_mtime_keeps_older_archive_time() {
    setup();

    fs::create_dir_all("extract_clamp_mtime_older/in").unwrap();
    fs::write("extract_clamp_mtime_older/in/file.txt", "content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "extract_clamp_mtime_older/archive.pna",
        "--overwrite",
        "extract_clamp_mtime_older/in/file.txt",
        "--keep-timestamp",
        "--mtime",
        "2020-01-15T12:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_clamp_mtime_older/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_clamp_mtime_older/out",
        "--mtime",
        "2025-01-01T00:00:00Z",
        "--clamp-mtime",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mtime =
        extract_mtime("extract_clamp_mtime_older/out/extract_clamp_mtime_older/in/file.txt");
    let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(1579089600); // 2020-01-15T12:00:00Z
    assert_same_second(mtime, expected, "clamp keeps older archive time");
}

/// Precondition: Archive contains a file with mtime 2020-01-15.
/// Action: Extract with `--mtime 2019-01-01 --clamp-mtime`.
/// Expectation: Extracted file has mtime 2019-01-01 (clamp value is older, so it's used).
#[test]
fn extract_with_clamp_mtime_uses_older_clamp_value() {
    setup();

    fs::create_dir_all("extract_clamp_mtime_clamp/in").unwrap();
    fs::write("extract_clamp_mtime_clamp/in/file.txt", "content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "extract_clamp_mtime_clamp/archive.pna",
        "--overwrite",
        "extract_clamp_mtime_clamp/in/file.txt",
        "--keep-timestamp",
        "--mtime",
        "2020-01-15T12:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_clamp_mtime_clamp/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_clamp_mtime_clamp/out",
        "--mtime",
        "2019-01-01T00:00:00Z",
        "--clamp-mtime",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mtime =
        extract_mtime("extract_clamp_mtime_clamp/out/extract_clamp_mtime_clamp/in/file.txt");
    let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(1546300800); // 2019-01-01
    assert_same_second(mtime, expected, "clamp uses older clamp value");
}

/// Precondition: Archive contains a file with atime 2020-01-15.
/// Action: Extract with `--atime 2024-06-01` (override mode).
/// Expectation: Extracted file has atime 2024-06-01.
#[test]
fn extract_with_atime_override() {
    setup();

    fs::create_dir_all("extract_atime_override/in").unwrap();
    fs::write("extract_atime_override/in/file.txt", "content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "extract_atime_override/archive.pna",
        "--overwrite",
        "extract_atime_override/in/file.txt",
        "--keep-timestamp",
        "--atime",
        "2020-01-15T12:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_atime_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_atime_override/out",
        "--atime",
        "2024-06-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let atime = fs::metadata("extract_atime_override/out/extract_atime_override/in/file.txt")
        .unwrap()
        .accessed()
        .unwrap();
    let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(1717200000); // 2024-06-01
    assert_same_second(atime, expected, "atime override");
}

/// Precondition: Archive contains a file with ctime 2020-01-15.
/// Action: Extract with `--ctime 2024-06-01` (override mode).
/// Expectation: On supported platforms (Windows/macOS), extracted file has ctime 2024-06-01.
#[test]
#[cfg(any(windows, target_os = "macos"))]
fn extract_with_ctime_override() {
    setup();

    fs::create_dir_all("extract_ctime_override/in").unwrap();
    fs::write("extract_ctime_override/in/file.txt", "content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "extract_ctime_override/archive.pna",
        "--overwrite",
        "extract_ctime_override/in/file.txt",
        "--keep-timestamp",
        "--ctime",
        "2020-01-15T12:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_ctime_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_ctime_override/out",
        "--ctime",
        "2024-06-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let ctime = fs::metadata("extract_ctime_override/out/extract_ctime_override/in/file.txt")
        .unwrap()
        .created()
        .unwrap();
    let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(1717200000); // 2024-06-01
    assert_same_second(ctime, expected, "ctime override");
}

/// Precondition: Archive contains a file with specific timestamps.
/// Action: Extract with combined --mtime and --atime overrides.
/// Expectation: Both mtime and atime are overridden independently.
#[test]
fn extract_with_multiple_time_overrides() {
    setup();

    fs::create_dir_all("extract_multi_time_override/in").unwrap();
    fs::write("extract_multi_time_override/in/file.txt", "content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "extract_multi_time_override/archive.pna",
        "--overwrite",
        "extract_multi_time_override/in/file.txt",
        "--keep-timestamp",
        "--mtime",
        "2020-01-15T12:00:00Z",
        "--atime",
        "2020-01-15T12:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_multi_time_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_multi_time_override/out",
        "--mtime",
        "2024-06-01T00:00:00Z",
        "--atime",
        "2024-07-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta =
        fs::metadata("extract_multi_time_override/out/extract_multi_time_override/in/file.txt")
            .unwrap();
    let mtime = meta.modified().unwrap();
    let atime = meta.accessed().unwrap();

    let expected_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1717200000); // 2024-06-01
    let expected_atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1719792000); // 2024-07-01

    assert_same_second(mtime, expected_mtime, "mtime");
    assert_same_second(atime, expected_atime, "atime");
}

const ARCHIVED_MTIME: Duration = Duration::from_secs(1_579_046_400); // 2020-01-15
const ARCHIVED_ATIME: Duration = Duration::from_secs(1_579_132_800); // 2020-01-16

fn create_fixture(base: &str) -> String {
    let archive = format!("{base}/archive.pna");
    fs::create_dir_all(format!("{base}/in")).unwrap();
    fs::write(format!("{base}/in/file.txt"), "content").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        archive.as_str(),
        "--overwrite",
        &format!("{base}/in/file.txt"),
        "--keep-timestamp",
        "--mtime",
        "2020-01-15T00:00:00Z",
        "--atime",
        "2020-01-16T00:00:00Z",
        "--ctime",
        "2020-01-17T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive
}

fn extract_fixture(base: &str, archive: &str, flags: &[&str]) -> String {
    let out_dir = format!("{base}/out");
    let mut args = vec![
        "pna",
        "--quiet",
        "x",
        "-f",
        archive,
        "--overwrite",
        "--out-dir",
        out_dir.as_str(),
    ];
    args.extend_from_slice(flags);
    cli::Cli::try_parse_from(args).unwrap().execute().unwrap();
    format!("{out_dir}/{base}/in/file.txt")
}

/// Precondition: An archive stores a file with explicit mtime, ctime, and atime.
/// Action: Extract with `--mtime` but no preservation flag.
/// Expectation: The extracted file overrides mtime and implicitly restores the other archived timestamps.
#[test]
fn extract_with_mtime_implicitly_preserves_other_timestamps() {
    setup();
    let base = "extract_with_mtime_implicitly_preserves_other_timestamps";
    let archive = create_fixture(base);

    let extracted = extract_fixture(base, &archive, &["--mtime", "2024-06-01T00:00:00Z"]);

    let meta = fs::metadata(extracted).unwrap();
    assert_same_second(
        meta.modified().unwrap(),
        SystemTime::UNIX_EPOCH + Duration::from_secs(1717200000),
        "mtime",
    );
    assert_same_second(
        meta.accessed().unwrap(),
        SystemTime::UNIX_EPOCH + ARCHIVED_ATIME,
        "atime",
    );
}

/// Precondition: An archive stores a file with explicit mtime, ctime, and atime.
/// Action: Extract with `--atime` but no preservation flag.
/// Expectation: The extracted file overrides atime and implicitly restores the other archived timestamps.
#[test]
fn extract_with_atime_implicitly_preserves_other_timestamps() {
    setup();
    let base = "extract_with_atime_implicitly_preserves_other_timestamps";
    let archive = create_fixture(base);

    let extracted = extract_fixture(base, &archive, &["--atime", "2024-07-01T00:00:00Z"]);

    let meta = fs::metadata(extracted).unwrap();
    assert_same_second(
        meta.modified().unwrap(),
        SystemTime::UNIX_EPOCH + ARCHIVED_MTIME,
        "mtime",
    );
    assert_same_second(
        meta.accessed().unwrap(),
        SystemTime::UNIX_EPOCH + Duration::from_secs(1719792000),
        "atime",
    );
}

/// Precondition: An archive stores a file with explicit mtime, ctime, and atime.
/// Action: Extract with `--ctime` but no preservation flag.
/// Expectation: The extracted file keeps the archived mtime and atime; on supported platforms ctime is overridden.
#[test]
fn extract_with_ctime_keeps_other_timestamps() {
    setup();
    let base = "extract_with_ctime_keeps_other_timestamps";
    let archive = create_fixture(base);

    let extracted = extract_fixture(base, &archive, &["--ctime", "2024-06-01T00:00:00Z"]);

    let meta = fs::metadata(extracted).unwrap();
    assert_same_second(
        meta.modified().unwrap(),
        SystemTime::UNIX_EPOCH + ARCHIVED_MTIME,
        "mtime",
    );
    assert_same_second(
        meta.accessed().unwrap(),
        SystemTime::UNIX_EPOCH + ARCHIVED_ATIME,
        "atime",
    );
    #[cfg(any(windows, target_os = "macos"))]
    assert_same_second(
        meta.created().unwrap(),
        SystemTime::UNIX_EPOCH + Duration::from_secs(1717200000),
        "ctime",
    );
}

/// Precondition: An archive stores a file with explicit mtime, ctime, and atime.
/// Action: Extract with `--mtime` and `--clamp-mtime` where the clamp is older than the archived mtime.
/// Expectation: The extracted file uses the clamp value and keeps the archived atime.
#[test]
fn extract_with_clamp_mtime_keeps_other_timestamps() {
    setup();
    let base = "extract_with_clamp_mtime_keeps_other_timestamps";
    let archive = create_fixture(base);

    let extracted = extract_fixture(
        base,
        &archive,
        &["--mtime", "2019-01-01T00:00:00Z", "--clamp-mtime"],
    );

    let meta = fs::metadata(extracted).unwrap();
    assert_same_second(
        meta.modified().unwrap(),
        SystemTime::UNIX_EPOCH + Duration::from_secs(1546300800),
        "mtime",
    );
    assert_same_second(
        meta.accessed().unwrap(),
        SystemTime::UNIX_EPOCH + ARCHIVED_ATIME,
        "atime",
    );
}
