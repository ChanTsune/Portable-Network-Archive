use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn create_hardlink_test_archive(base: &Path) {
    // Clean up any existing test directory
    let _ = fs::remove_dir_all(base);

    let input = base.join("in");
    fs::create_dir_all(&input).unwrap();

    // Create original file and hard link
    fs::write(input.join("origin.txt"), b"hardlink test content").unwrap();
    fs::hard_link(input.join("origin.txt"), input.join("link.txt")).unwrap();

    let archive = base.join("test.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "-f",
        archive.to_str().unwrap(),
        "--overwrite",
        input.to_str().unwrap(),
    ])
    .assert()
    .success();
}

/// Precondition: Archive contains a hard link entry pointing to origin.txt.
/// Action: Run `pna experimental diff` when both files exist and are properly linked.
/// Expectation: No difference reported.
#[test]
fn diff_hardlink_no_diff() {
    setup();
    let base = Path::new("diff_hardlink_no_diff");
    create_hardlink_test_archive(base);

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "experimental",
            "diff",
            "-f",
            base.join("test.pna").to_str().unwrap(),
        ])
        .assert();

    // No output means no differences
    assert.success().stdout("");
}

/// Precondition: Archive contains hard-linked files (origin.txt and link.txt).
/// Action: Delete one of the hard-linked files and run `pna experimental diff`.
/// Expectation: Reports missing file or hard link (depending on which was stored as link).
/// Note: Directory iteration order differs between OSes, so either file may be the "original".
#[test]
fn diff_hardlink_missing_link() {
    setup();
    let base = Path::new("diff_hardlink_missing_link");
    create_hardlink_test_archive(base);

    // Remove the hard link file
    fs::remove_file(base.join("in/link.txt")).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "experimental",
            "diff",
            "-f",
            base.join("test.pna").to_str().unwrap(),
        ])
        .assert();

    // On Unix: link.txt is stored as hard link → "Missing hard link:"
    // On Windows: link.txt may be stored as file → "Missing file:"
    assert.success().stdout(
        predicate::str::contains("Missing hard link:").or(predicate::str::contains("Missing file:")),
    );
}

/// Precondition: Archive contains hard-linked files (origin.txt and link.txt).
/// Action: Delete one file and replace the other with a regular file (breaking the link).
/// Expectation: Reports missing file/hard link (depending on archive order).
/// Note: Directory iteration order differs between OSes, so either file may be the "original".
#[test]
fn diff_hardlink_missing_target() {
    setup();
    let base = Path::new("diff_hardlink_missing_target");
    create_hardlink_test_archive(base);

    // Remove the original file but keep the link
    // Since they share the same inode, we need to recreate as a regular file
    let link_content = fs::read(base.join("in/link.txt")).unwrap();
    fs::remove_file(base.join("in/origin.txt")).unwrap();
    fs::remove_file(base.join("in/link.txt")).unwrap();
    fs::write(base.join("in/link.txt"), link_content).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "experimental",
            "diff",
            "-f",
            base.join("test.pna").to_str().unwrap(),
        ])
        .assert();

    // On Unix: origin.txt is file, link.txt is hard link → "Hard link mismatch:" for link.txt
    // On Windows: link.txt may be file, origin.txt may be hard link → "Missing hard link:" for origin.txt
    assert.success().stdout(
        predicate::str::contains("Hard link mismatch:")
            .or(predicate::str::contains("Missing hard link:")),
    );
}

/// Precondition: Archive contains a hard link entry pointing to origin.txt.
/// Action: Replace the hard link with a regular file (same content but different inode).
/// Expectation: Reports that the file is not linked to the target.
#[test]
fn diff_hardlink_broken_link() {
    setup();
    let base = Path::new("diff_hardlink_broken_link");
    create_hardlink_test_archive(base);

    // Replace hard link with a regular file (same content, different inode)
    let content = fs::read(base.join("in/link.txt")).unwrap();
    fs::remove_file(base.join("in/link.txt")).unwrap();
    fs::write(base.join("in/link.txt"), content).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "experimental",
            "diff",
            "-f",
            base.join("test.pna").to_str().unwrap(),
        ])
        .assert();

    assert
        .success()
        .stdout(predicate::str::contains("Hard link mismatch:"));
}

/// Precondition: Archive contains a hard link entry.
/// Action: Replace the hard link with a directory.
/// Expectation: Reports file type mismatch.
#[test]
fn diff_hardlink_type_mismatch() {
    setup();
    let base = Path::new("diff_hardlink_type_mismatch");
    create_hardlink_test_archive(base);

    // Replace hard link with a directory
    fs::remove_file(base.join("in/link.txt")).unwrap();
    fs::create_dir(base.join("in/link.txt")).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "experimental",
            "diff",
            "-f",
            base.join("test.pna").to_str().unwrap(),
        ])
        .assert();

    assert
        .success()
        .stdout(predicate::str::contains("Mismatch file type:"));
}
