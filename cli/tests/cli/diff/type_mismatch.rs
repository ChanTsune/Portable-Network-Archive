use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;

/// Precondition: Archive contains symlink.
/// Action: Replace symlink with regular file on the filesystem, run diff.
/// Expectation: Reports "File type mismatch".
#[cfg(unix)]
#[test]
fn diff_detects_symlink_to_file_mismatch() {
    setup();
    let dir = "diff_type_mismatch_symlink_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let target = format!("{dir}/target");
    let link = format!("{dir}/link");
    fs::write(&target, "content").unwrap();
    symlink("target", &link).unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["create", "-f", &archive_path, "--overwrite", &link])
        .assert()
        .success();

    fs::remove_file(&link).unwrap();
    fs::write(&link, "regular file content").unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "{link}: File type mismatch"
        )));
}

/// Precondition: Archive contains regular file.
/// Action: Replace file with directory on the filesystem, run diff.
/// Expectation: Reports "File type mismatch".
#[test]
fn diff_detects_file_to_directory_mismatch() {
    setup();
    let dir = "diff_type_mismatch_file_dir_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/item");
    fs::write(&file_path, "content").unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::remove_file(&file_path).unwrap();
    fs::create_dir(&file_path).unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "{file_path}: File type mismatch"
        )));
}

/// Precondition: Archive contains directory.
/// Action: Replace directory with regular file on the filesystem, run diff.
/// Expectation: Reports "File type mismatch".
#[test]
fn diff_detects_directory_to_file_mismatch() {
    setup();
    let dir = "diff_type_mismatch_dir_file_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let subdir = format!("{dir}/item");
    fs::create_dir(&subdir).unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "create",
        "-f",
        &archive_path,
        "--overwrite",
        "--keep-dir",
        &subdir,
    ])
    .assert()
    .success();

    fs::remove_dir(&subdir).unwrap();
    fs::write(&subdir, "now a file").unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "{subdir}: File type mismatch"
        )));
}
