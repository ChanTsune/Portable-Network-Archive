use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

/// Precondition: Archive contains multiple files, one is modified on disk.
/// Action: Run diff with path argument specifying only unmodified file.
/// Expectation: No differences reported.
#[test]
fn diff_filters_by_path_argument() {
    setup();
    let dir = "diff_path_filter_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_a = format!("{dir}/a.txt");
    let file_b = format!("{dir}/b.txt");
    fs::write(&file_a, "content a").unwrap();
    fs::write(&file_b, "content b").unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "create",
        "-f",
        &archive_path,
        "--overwrite",
        &file_a,
        &file_b,
    ])
    .assert()
    .success();

    // Modify file_a
    fs::write(&file_a, "modified").unwrap();

    // Without filter: reports difference
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .stdout(predicate::str::contains("Size differs"));

    // With filter for file_b only: no difference
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["experimental", "diff", "-f", &archive_path, &file_b])
        .assert()
        .stdout("");
}

/// Precondition: Archive exists with files.
/// Action: Run diff with path argument that doesn't exist in archive.
/// Expectation: Reports "not found in archive" and exits with error.
#[test]
fn diff_reports_path_not_in_archive() {
    setup();
    let dir = "diff_not_found_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file = format!("{dir}/exists.txt");
    fs::write(&file, "content").unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["create", "-f", &archive_path, "--overwrite", &file])
        .assert()
        .success();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "diff",
        "-f",
        &archive_path,
        "nonexistent.txt",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("not found in archive"));
}

/// Precondition: Archive contains directory with files.
/// Action: Run diff with directory path argument.
/// Expectation: All files under directory are compared.
#[test]
fn diff_filters_by_directory_prefix() {
    setup();
    let dir = "diff_dir_prefix_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(format!("{dir}/subdir")).unwrap();

    let file_a = format!("{dir}/a.txt");
    let file_b = format!("{dir}/subdir/b.txt");
    fs::write(&file_a, "content a").unwrap();
    fs::write(&file_b, "content b").unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "create",
        "-f",
        &archive_path,
        "--overwrite",
        &file_a,
        &file_b,
    ])
    .assert()
    .success();

    // Modify both files
    fs::write(&file_a, "modified a").unwrap();
    fs::write(&file_b, "modified b").unwrap();

    // Filter by subdir: only reports subdir/b.txt difference
    let subdir = format!("{dir}/subdir");
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args(["experimental", "diff", "-f", &archive_path, &subdir])
        .assert();

    assert
        .stdout(predicate::str::contains("subdir/b.txt: Size differs"))
        .stdout(predicate::str::contains("a.txt: Size differs").not());
}
