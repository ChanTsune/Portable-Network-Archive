use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

/// Precondition: A solid archive matches the filesystem exactly.
/// Action: Run `pna experimental diff`.
/// Expectation: Exits successfully and produces no stdout.
#[test]
fn diff_solid_without_differences() {
    setup();
    let dir = "diff_solid_without_differences_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            &file_path,
            "--solid",
        ])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout("");
}

/// Precondition: A solid archive contains a file whose content is later changed on disk to a different value of the same size.
/// Action: Run `pna experimental diff`.
/// Expectation: Exits with status 1 and reports the difference on stdout, with nothing written to stderr.
#[test]
fn diff_solid_with_content_difference() {
    setup();
    let dir = "diff_solid_with_content_difference_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            &file_path,
            "--solid",
        ])
        .assert()
        .success();

    fs::write(&file_path, "new-a").unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Contents differ"))
        .stderr("");
}
