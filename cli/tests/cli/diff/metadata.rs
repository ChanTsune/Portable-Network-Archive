#[cfg(unix)]
use crate::utils::setup;
#[cfg(unix)]
use assert_cmd::cargo::cargo_bin_cmd;
#[cfg(unix)]
use predicates::prelude::*;
#[cfg(unix)]
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
        .code(1)
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
        .code(1)
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
        .code(1)
        .stdout(predicate::str::contains("Mod time differs"));
}

/// Precondition: Archive stores a file mtime with zero sub-second precision.
/// Action: Filesystem mtime is set to the same whole second with nonzero nanoseconds.
/// Expectation: No "Mod time differs" is reported; whole-second storage compares by whole seconds.
/// Returns early on filesystems without nanosecond timestamp support.
#[cfg(unix)]
#[test]
fn diff_ignores_subsecond_when_archive_has_whole_second_mtime() {
    setup();
    let dir = "diff_mtime_ignores_subsecond_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let whole_second = SystemTime::UNIX_EPOCH + Duration::from_secs(86400);
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_system_time(whole_second),
    )
    .unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            &file_path,
        ])
        .assert()
        .success();

    let with_nanos = whole_second + Duration::from_nanos(123_456_789);
    filetime::set_file_mtime(&file_path, filetime::FileTime::from_system_time(with_nanos)).unwrap();
    if fs::symlink_metadata(&file_path)
        .unwrap()
        .modified()
        .unwrap()
        != with_nanos
    {
        return;
    }

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout("");
}

/// Precondition: Archive stores a file mtime with nonzero sub-second precision.
/// Action: Filesystem mtime is the same whole second but with different nanoseconds.
/// Expectation: Reports "Mod time differs"; sub-second storage requires exact equality.
/// Returns early on filesystems without nanosecond timestamp support.
#[cfg(unix)]
#[test]
fn diff_detects_subsecond_difference_when_archive_stores_nanoseconds() {
    setup();
    let dir = "diff_mtime_detects_subsecond_diff_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let whole_second = SystemTime::UNIX_EPOCH + Duration::from_secs(86400);
    let archived_time = whole_second + Duration::from_nanos(123_456_789);
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_system_time(archived_time),
    )
    .unwrap();
    if fs::symlink_metadata(&file_path)
        .unwrap()
        .modified()
        .unwrap()
        != archived_time
    {
        return;
    }

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            &file_path,
        ])
        .assert()
        .success();

    let different_nanos = whole_second + Duration::from_nanos(987_654_321);
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_system_time(different_nanos),
    )
    .unwrap();
    if fs::symlink_metadata(&file_path)
        .unwrap()
        .modified()
        .unwrap()
        != different_nanos
    {
        return;
    }

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Mod time differs"));
}

/// Precondition: Archive stores a file mtime with nonzero sub-second precision.
/// Action: Filesystem mtime is left unchanged (exact nanosecond roundtrip).
/// Expectation: No "Mod time differs" is reported.
/// Returns early on filesystems without nanosecond timestamp support.
#[cfg(unix)]
#[test]
fn diff_accepts_exact_nanosecond_roundtrip() {
    setup();
    let dir = "diff_mtime_exact_roundtrip_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let archived_time =
        SystemTime::UNIX_EPOCH + Duration::from_secs(86400) + Duration::from_nanos(123_456_789);
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_system_time(archived_time),
    )
    .unwrap();
    if fs::symlink_metadata(&file_path)
        .unwrap()
        .modified()
        .unwrap()
        != archived_time
    {
        return;
    }

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            &file_path,
        ])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout("");
}

/// Precondition: Archive stores a file mtime with zero sub-second precision.
/// Action: Filesystem mtime is set to the same whole second, at its last nanosecond.
/// Expectation: No "Mod time differs" is reported (boundary: still within the same whole second).
/// Returns early on filesystems without nanosecond timestamp support.
#[cfg(unix)]
#[test]
fn diff_ignores_end_of_second_when_archive_has_whole_second_mtime() {
    setup();
    let dir = "diff_mtime_boundary_end_of_second_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let whole_second = SystemTime::UNIX_EPOCH + Duration::from_secs(86400);
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_system_time(whole_second),
    )
    .unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            &file_path,
        ])
        .assert()
        .success();

    let end_of_second = whole_second + Duration::from_nanos(999_999_999);
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_system_time(end_of_second),
    )
    .unwrap();
    if fs::symlink_metadata(&file_path)
        .unwrap()
        .modified()
        .unwrap()
        != end_of_second
    {
        return;
    }

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout("");
}

/// Precondition: Archive stores a file mtime with zero sub-second precision.
/// Action: Filesystem mtime is set to exactly one whole second later.
/// Expectation: Reports "Mod time differs" (boundary: crosses into the next whole second).
#[cfg(unix)]
#[test]
fn diff_detects_next_second_when_archive_has_whole_second_mtime() {
    setup();
    let dir = "diff_mtime_boundary_next_second_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let whole_second = SystemTime::UNIX_EPOCH + Duration::from_secs(86400);
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_system_time(whole_second),
    )
    .unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            &file_path,
        ])
        .assert()
        .success();

    let next_second = whole_second + Duration::from_secs(1);
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_system_time(next_second),
    )
    .unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Mod time differs"));
}
