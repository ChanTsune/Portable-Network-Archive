use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;

/// Precondition: Archive contains symlink pointing to "target".
/// Action: Change symlink to point to "other" on filesystem, run diff.
/// Expectation: Reports "Symlink differs".
#[cfg(unix)]
#[test]
fn diff_detects_symlink_change() {
    setup();
    let dir = "diff_symlink_test";
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

    // Change symlink target
    fs::remove_file(&link).unwrap();
    symlink("other", &link).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args(["experimental", "diff", "-f", &archive_path])
        .assert();

    assert.stdout(predicate::str::contains("Symlink differs"));
}
