#![cfg(not(target_family = "wasm"))]
use crate::utils::{diff::diff, setup, TestResources};
use assert_cmd::Command;

#[test]
fn test_update_files_from_stdin() {
    setup();
    TestResources::extract_in("raw/images/", "update_files_from_stdin/in/").unwrap();
    TestResources::extract_in("raw/parent/", "update_files_from_stdin/in/").unwrap();
    TestResources::extract_in("raw/pna/", "update_files_from_stdin/in/").unwrap();

    // Create a base archive
    Command::cargo_bin("pna")
        .unwrap()
        .args(&[
            "--quiet",
            "create",
            "update_files_from_stdin/base.pna",
            "--overwrite",
            "update_files_from_stdin/in/raw/pna/",
        ])
        .assert()
        .success();

    // Prepare a file list for stdin
    let file_list =
        "update_files_from_stdin/in/raw/images/\nupdate_files_from_stdin/in/raw/parent/";

    // Run update command with --files-from-stdin
    let mut cmd = Command::cargo_bin("pna").unwrap();
    cmd.args(&[
        "--quiet",
        "experimental",
        "update",
        "update_files_from_stdin/base.pna",
        "--files-from-stdin",
        "--unstable",
    ])
    .write_stdin(file_list);

    cmd.assert().success();

    // Extract the updated archive and verify contents
    Command::cargo_bin("pna")
        .unwrap()
        .args(&[
            "--quiet",
            "extract",
            "update_files_from_stdin/base.pna",
            "--overwrite",
            "--out-dir",
            "update_files_from_stdin/out/",
        ])
        .assert()
        .success();

    // Check if expected files exist after extraction
    diff(
        "update_files_from_stdin/in/",
        "update_files_from_stdin/out/",
    )
    .unwrap();
}
