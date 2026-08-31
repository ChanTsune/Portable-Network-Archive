#![cfg(not(target_family = "wasm"))]

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: An archive exists, and a run is given a path pattern that matches nothing.
/// Action: Run the deletion with a dedicated temp directory in the environment.
/// Expectation: The run fails and leaves no staged file behind.
#[test]
fn delete_leaves_no_staged_file_when_a_pattern_is_missing() {
    setup();
    let _ = fs::remove_dir_all("delete_staged_cleanup");
    TestResources::extract_in("raw/", "delete_staged_cleanup/in/").unwrap();
    fs::create_dir_all("delete_staged_cleanup/tmp").unwrap();
    let temp_dir = fs::canonicalize("delete_staged_cleanup/tmp").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "-f",
            "delete_staged_cleanup/archive.pna",
            "--overwrite",
            "delete_staged_cleanup/in/",
        ])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .env("TMPDIR", &temp_dir)
        .env("TMP", &temp_dir)
        .env("TEMP", &temp_dir)
        .args([
            "--quiet",
            "delete",
            "--overwrite",
            "-f",
            "delete_staged_cleanup/archive.pna",
            "delete_staged_cleanup/in/raw/not_found.txt",
        ])
        .assert()
        .failure();

    let left = fs::read_dir(&temp_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    assert!(left.is_empty(), "staged file left behind: {left:?}");
}
