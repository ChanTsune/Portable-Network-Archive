use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::time::{Duration, SystemTime};

/// Precondition: Archive contains file with mode 0644.
/// Action: Change file mode to 0755 on the filesystem, run diff.
/// Expectation: Reports "Mode differs".
#[cfg(unix)]
#[test]
fn diff_detects_file_mode_change() {
    setup();
    let dir = "diff_file_mode_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();
    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "create",
        "-f",
        &archive_path,
        "--overwrite",
        "--keep-permission",
        &file_path,
    ])
    .assert()
    .success();

    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o755)).unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .stdout(predicate::str::contains("Mode differs"));
}

/// Precondition: Archive contains directory with mode 0755.
/// Action: Change directory mode to 0700 on the filesystem, run diff.
/// Expectation: Reports "Mode differs" for directory.
#[cfg(unix)]
#[test]
fn diff_detects_directory_mode_change() {
    setup();
    let dir = "diff_dir_mode_test";
    let _ = fs::remove_dir_all(dir);
    let subdir = format!("{dir}/subdir");
    fs::create_dir_all(&subdir).unwrap();
    fs::set_permissions(&subdir, fs::Permissions::from_mode(0o755)).unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "create",
        "-f",
        &archive_path,
        "--overwrite",
        "--keep-permission",
        "--keep-dir",
        &subdir,
    ])
    .assert()
    .success();

    fs::set_permissions(&subdir, fs::Permissions::from_mode(0o700)).unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .stdout(predicate::str::contains("Mode differs"));
}

/// Precondition: Archive contains directory with specific mtime.
/// Action: Change directory mtime on the filesystem, run diff without --full-compare.
/// Expectation: No mtime difference reported (default behavior ignores directory mtime).
#[cfg(unix)]
#[test]
fn diff_ignores_directory_mtime_by_default() {
    setup();
    let dir = "diff_dir_mtime_default_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let subdir = format!("{dir}/subdir");
    fs::create_dir_all(&subdir).unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "create",
        "-f",
        &archive_path,
        "--overwrite",
        "--keep-permission",
        "--keep-timestamp",
        "--keep-dir",
        &subdir,
    ])
    .assert()
    .success();

    let new_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(86400); // 1970-01-02
    filetime::set_file_mtime(&subdir, filetime::FileTime::from_system_time(new_mtime)).unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mod time differs").not())
        .stdout(predicate::str::contains("Mode differs").not());
}

/// Precondition: Archive contains directory with specific mtime.
/// Action: Change directory mtime on the filesystem, run diff with --full-compare.
/// Expectation: Reports "Mod time differs" for directory.
#[cfg(unix)]
#[test]
fn diff_detects_directory_mtime_with_full_compare() {
    setup();
    let dir = "diff_dir_mtime_full_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let subdir = format!("{dir}/subdir");
    fs::create_dir_all(&subdir).unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "create",
        "-f",
        &archive_path,
        "--overwrite",
        "--keep-permission",
        "--keep-timestamp",
        "--keep-dir",
        &subdir,
    ])
    .assert()
    .success();

    let new_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(86400); // 1970-01-02
    filetime::set_file_mtime(&subdir, filetime::FileTime::from_system_time(new_mtime)).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--full-compare",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mod time differs"));
}
