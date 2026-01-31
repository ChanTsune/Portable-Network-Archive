use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

/// Precondition: Archive contains hardlink.
/// Action: Replace hardlink with directory on the filesystem, run diff.
/// Expectation: Reports "File type mismatch".
#[cfg(unix)]
#[test]
fn diff_detects_hardlink_to_directory_mismatch() {
    setup();
    let dir = "diff_hardlink_dir_mismatch_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let orig = format!("{dir}/orig.txt");
    let link = format!("{dir}/link.txt");
    fs::write(&orig, "content").unwrap();
    fs::hard_link(&orig, &link).unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &orig, &link])
        .assert()
        .success();

    fs::remove_file(&link).unwrap();
    fs::create_dir(&link).unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "{link}: File type mismatch"
        )));
}

/// Precondition: Archive contains hardlink.
/// Action: Replace hardlink with regular file on the filesystem, run diff.
/// Expectation: Reports "Not linked to".
#[cfg(unix)]
#[test]
fn diff_detects_broken_hardlink() {
    setup();
    let dir = "diff_hardlink_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let orig = format!("{dir}/orig.txt");
    let link = format!("{dir}/link.txt");
    fs::write(&orig, "content").unwrap();
    fs::hard_link(&orig, &link).unwrap();

    let archive_path = format!("{dir}/test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["create", "-f", &archive_path, "--overwrite", &orig, &link])
        .assert()
        .success();

    fs::remove_file(&link).unwrap();
    fs::write(&link, "content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args(["experimental", "diff", "-f", &archive_path])
        .assert();

    assert.stdout(predicate::str::contains("Not linked to"));
}
