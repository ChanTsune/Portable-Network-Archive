use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: the source tree contains file and directory.
/// Action: run `pna create` with `--exclude` to build an archive, then compare by
///         `pna experimental diff`.
/// Expectation: no difference detected.
#[test]
fn diff_missing_in_archive() {
    setup();
    TestResources::extract_in("raw/", "diff_missing_in_archive/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "-f",
        "diff_missing_in_archive/diff.pna",
        "--overwrite",
        "diff_missing_in_archive/in/",
        "--exclude",
        "*.svg",
        "--unstable",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "experimental",
            "diff",
            "-f",
            "diff_missing_in_archive/diff.pna",
        ])
        .assert();

    assert.stdout("");
}

/// Precondition: the source tree contains file and directory.
/// Action: run `pna create` to build an archive and remove *.svg from filesystem, then compare by
///         `pna experimental diff`.
/// Expectation: detect missing file.
#[test]
fn diff_missing_in_disk() {
    setup();
    TestResources::extract_in("raw/", "diff_missing_in_disk/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "-f",
        "diff_missing_in_disk/diff.pna",
        "--overwrite",
        "diff_missing_in_disk/in/",
    ])
    .assert()
    .success();
    fs::remove_file("diff_missing_in_disk/in/raw/images/icon.svg").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "experimental",
            "diff",
            "-f",
            "diff_missing_in_disk/diff.pna",
        ])
        .assert();

    assert.stdout(
        "diff_missing_in_disk/in/raw/images/icon.svg: Warning: Cannot stat: No such file or directory\n",
    );
}
