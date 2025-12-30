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
/// Expectation: Extracted file has mtime 2024-06-01, ignoring archive timestamp.
#[test]
fn extract_with_mtime_override() {
    setup();

    fs::create_dir_all("extract_mtime_override/in").unwrap();
    fs::write("extract_mtime_override/in/file.txt", "content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
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
