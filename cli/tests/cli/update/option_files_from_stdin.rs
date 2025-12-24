#![cfg(not(target_family = "wasm"))]
use crate::utils::{EmbedExt, TestResources, diff::diff, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: An archive exists with some files.
/// Action: Run `pna experimental update` with `--files-from-stdin` providing additional paths.
/// Expectation: Files from stdin are added to the archive.
#[test]
fn update_with_files_from_stdin() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_files_from_stdin");
    TestResources::extract_in("raw/images/", "update_files_from_stdin/in/").unwrap();
    TestResources::extract_in("raw/parent/", "update_files_from_stdin/in/").unwrap();
    TestResources::extract_in("raw/pna/", "update_files_from_stdin/in/").unwrap();

    // Create a base archive
    cargo_bin_cmd!("pna")
        .args([
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
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_files_from_stdin/base.pna",
        "--files-from-stdin",
        "--unstable",
    ])
    .write_stdin(file_list);

    cmd.assert().success();

    // Extract the updated archive and verify contents
    cargo_bin_cmd!("pna")
        .args([
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
