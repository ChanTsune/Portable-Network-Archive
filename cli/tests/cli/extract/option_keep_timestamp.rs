use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

fn extract_time(path: impl AsRef<Path>) -> (SystemTime, SystemTime, Option<SystemTime>) {
    let meta = fs::metadata(path).unwrap();
    let modified = meta.modified().unwrap();
    let accessed = meta.accessed().unwrap();
    let created = meta.created().ok();
    (modified, accessed, created)
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

/// Precondition: Create an archive that stores a file with explicitly specified atime/mtime/ctime.
/// Action: Extract via `pna extract` with `--keep-timestamp` into an empty output directory.
/// Expectation: The extracted file has the same timestamps as recorded in the archive.
#[test]
fn extract_keep_timestamp_restores_file_times() {
    setup();

    fs::create_dir_all("extract_keep_timestamp/in").unwrap();
    fs::write("extract_keep_timestamp/in/file.txt", "content").unwrap();

    let atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1704067200);
    let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1704153600);
    #[cfg(any(windows, target_os = "macos"))]
    let ctime = SystemTime::UNIX_EPOCH + Duration::from_secs(1704240000);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_keep_timestamp/archive.pna",
        "--overwrite",
        "extract_keep_timestamp/in/file.txt",
        "--keep-timestamp",
        "--atime",
        "2024-01-01T00:00:00Z",
        "--mtime",
        "2024-01-02T00:00:00Z",
        "--ctime",
        "2024-01-03T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_keep_timestamp/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_keep_timestamp/out",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let (modified, accessed, _created) =
        extract_time("extract_keep_timestamp/out/extract_keep_timestamp/in/file.txt");
    assert_same_second(accessed, atime, "atime");
    assert_same_second(modified, mtime, "mtime");

    #[cfg(any(windows, target_os = "macos"))]
    assert_same_second(
        _created.expect("creation time should be available on this platform"),
        ctime,
        "ctime",
    );
}
