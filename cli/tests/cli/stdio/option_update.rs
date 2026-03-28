#![cfg(not(target_family = "wasm"))]
use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::predicate;
use std::{fs, thread, time};

/// Precondition: An archive exists with a file.
/// Action: Modify the file, run stdio update mode.
/// Expectation: The archive is updated with the newer file.
#[test]
fn stdio_update_basic() {
    setup();
    let archive = "stdio_update_basic/test.pna";
    let file = "stdio_update_basic/file.txt";
    fs::create_dir_all("stdio_update_basic").unwrap();

    // Create initial file
    fs::write(file, "original content").unwrap();

    // Create archive using pna create
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("c")
        .arg(archive)
        .arg("--overwrite")
        .arg("--keep-timestamp")
        .arg(file)
        .assert()
        .success();

    // Wait to ensure distinct mtime
    thread::sleep(time::Duration::from_millis(10));

    // Modify the file
    fs::write(file, "modified content").unwrap();

    // Update using stdio subcommand
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("--update")
        .arg("-f")
        .arg(archive)
        .arg("--keep-timestamp")
        .arg(file)
        .assert()
        .success();

    // Verify the archive was updated
    let mut found = false;
    archive::for_each_entry(archive, |entry| {
        if entry.header().path().to_string().ends_with("file.txt") {
            found = true;
        }
    })
    .unwrap();
    assert!(found, "file.txt should be in the updated archive");
}

/// Precondition: An archive exists with a file.
/// Action: Run stdio update mode with short flag `-u`.
/// Expectation: The update succeeds with the short flag.
#[test]
fn stdio_update_short_flag() {
    setup();
    let archive = "stdio_update_short_flag/test.pna";
    let file = "stdio_update_short_flag/file.txt";
    fs::create_dir_all("stdio_update_short_flag").unwrap();

    // Create initial file
    fs::write(file, "original content").unwrap();

    // Create archive
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("c")
        .arg(archive)
        .arg("--overwrite")
        .arg("--keep-timestamp")
        .arg(file)
        .assert()
        .success();

    // Wait to ensure distinct mtime
    thread::sleep(time::Duration::from_millis(10));

    // Modify the file
    fs::write(file, "modified content").unwrap();

    // Update using short flag
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-u")
        .arg("-f")
        .arg(archive)
        .arg("--keep-timestamp")
        .arg(file)
        .assert()
        .success();

    // Verify the archive was updated
    let mut found = false;
    archive::for_each_entry(archive, |entry| {
        if entry.header().path().to_string().ends_with("file.txt") {
            found = true;
        }
    })
    .unwrap();
    assert!(found, "file.txt should be in the updated archive");
}

/// Precondition: None.
/// Action: Run stdio update mode with stdin (`-f -`).
/// Expectation: Error is returned because update requires a file-based archive.
#[test]
fn stdio_update_requires_file() {
    setup();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-u")
        .arg("-f")
        .arg("-")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "update mode requires a file-based archive",
        ));
}

/// Precondition: None.
/// Action: Run stdio update mode without `-f` flag.
/// Expectation: Error is returned because update requires a file-based archive.
#[test]
fn stdio_update_requires_file_no_flag() {
    setup();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-u")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "update mode requires a file-based archive",
        ));
}

/// Precondition: An archive contains a file that hasn't been modified.
/// Action: Run stdio update without modifying the source.
/// Expectation: Archive entry is preserved, not re-added.
#[test]
fn stdio_update_preserves_unmodified() {
    setup();
    let archive = "stdio_update_preserves_unmodified/test.pna";
    let file = "stdio_update_preserves_unmodified/file.txt";
    fs::create_dir_all("stdio_update_preserves_unmodified").unwrap();

    // Create initial file
    fs::write(file, "content").unwrap();

    // Create archive
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("c")
        .arg(archive)
        .arg("--overwrite")
        .arg("--keep-timestamp")
        .arg(file)
        .assert()
        .success();

    // Run update without modifying the file
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-u")
        .arg("-f")
        .arg(archive)
        .arg("--keep-timestamp")
        .arg(file)
        .assert()
        .success();

    // Verify the archive has exactly one entry
    let mut count = 0;
    archive::for_each_entry(archive, |_| {
        count += 1;
    })
    .unwrap();
    assert_eq!(count, 1, "should have exactly one entry");
}
