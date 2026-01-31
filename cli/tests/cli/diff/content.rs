use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

/// Precondition: Archive contains file with specific content.
/// Action: Modify file to have different content but same size on the filesystem, run diff.
/// Expectation: Reports "Contents differ".
#[test]
fn diff_detects_content_change_same_size() {
    setup();
    let dir = "diff_content_same_size_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "aaaaa").unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::write(&file_path, "bbbbb").unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("Contents differ"));
}

/// Precondition: Archive contains file with specific content.
/// Action: Modify file to have different size on the filesystem, run diff.
/// Expectation: Reports "Size differs".
#[test]
fn diff_detects_size_change() {
    setup();
    let dir = "diff_size_change_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "short").unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::write(&file_path, "much longer content").unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("Size differs"));
}
