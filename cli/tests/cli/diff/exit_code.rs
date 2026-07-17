use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

/// Precondition: Archive matches the filesystem exactly.
/// Action: Run `pna experimental diff`.
/// Expectation: Exits with status 0 and produces no stdout.
#[test]
fn diff_without_differences_exits_zero() {
    setup();
    TestResources::extract_in("raw/", "diff_exit_code_zero/in/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "-f",
            "diff_exit_code_zero/diff.pna",
            "--overwrite",
            "diff_exit_code_zero/in/",
        ])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", "diff_exit_code_zero/diff.pna"])
        .assert()
        .success()
        .stdout("");
}

/// Precondition: Archive contains a file whose content is later changed on disk.
/// Action: Run `pna experimental diff`.
/// Expectation: Exits with status 1, reports the difference on stdout, and writes nothing to stderr.
#[test]
fn diff_with_content_difference_exits_one() {
    setup();
    let dir = "diff_exit_code_content_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "aaaaa").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::write(&file_path, "bbbbb").unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Contents differ"))
        .stderr("");
}

/// Precondition: Archive contains a file that is later removed from disk.
/// Action: Run `pna experimental diff`.
/// Expectation: Exits with status 1 and writes nothing to stderr.
#[test]
fn diff_with_missing_file_exits_one() {
    setup();
    let dir = "diff_exit_code_missing_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::remove_file(&file_path).unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Cannot stat"))
        .stderr("");
}

/// Precondition: The given archive path does not exist.
/// Action: Run `pna experimental diff`.
/// Expectation: Exits with status 2.
#[test]
fn diff_with_nonexistent_archive_exits_two() {
    setup();
    let dir = "diff_exit_code_nonexistent_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let archive_path = format!("{dir}/does_not_exist.pna");

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(2);
}
