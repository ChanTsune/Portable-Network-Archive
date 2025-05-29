#![cfg(not(target_family = "wasm"))]
use crate::utils::{components_count, diff::diff, setup, TestResources};
use assert_cmd::Command as Cmd;
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn delete_with_files_from_stdin() {
    setup();
    TestResources::extract_in("raw/", "delete_files_from_stdin/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_files_from_stdin/delete_files_from_stdin.pna",
        "--overwrite",
        "delete_files_from_stdin/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let list = ["**/raw/empty.txt", "**/raw/text.txt"].join("\n");

    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    cmd.write_stdin(list);
    cmd.args([
        "--quiet",
        "experimental",
        "delete",
        "delete_files_from_stdin/delete_files_from_stdin.pna",
        "--files-from-stdin",
        "--unstable",
    ]);
    cmd.assert().success();

    fs::remove_file("delete_files_from_stdin/in/raw/empty.txt").unwrap();
    fs::remove_file("delete_files_from_stdin/in/raw/text.txt").unwrap();

    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        "delete_files_from_stdin/delete_files_from_stdin.pna",
        "--overwrite",
        "--out-dir",
        "delete_files_from_stdin/out/",
        "--strip-components",
        &components_count("delete_files_from_stdin/in/").to_string(),
    ]);
    cmd.assert().success();

    diff(
        "delete_files_from_stdin/in/",
        "delete_files_from_stdin/out/",
    )
    .unwrap();
}
