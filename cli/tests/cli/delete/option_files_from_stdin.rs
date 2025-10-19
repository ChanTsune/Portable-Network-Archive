#![cfg(not(target_family = "wasm"))]
use crate::utils::{EmbedExt, TestResources, archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries by
///         `pna experimental delete` with `--files-from-stdin`.
/// Expectation: Only the entries named in the streamed list are removed; all other files remain.
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

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(list);
    cmd.args([
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_files_from_stdin/delete_files_from_stdin.pna",
        "--files-from-stdin",
        "--unstable",
    ]);
    cmd.assert().success();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "delete_files_from_stdin/delete_files_from_stdin.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "delete_files_from_stdin/in/raw/images/icon.svg",
        "delete_files_from_stdin/in/raw/images/icon.bmp",
        "delete_files_from_stdin/in/raw/pna/empty.pna",
        "delete_files_from_stdin/in/raw/pna/nest.pna",
        "delete_files_from_stdin/in/raw/images/icon.png",
        "delete_files_from_stdin/in/raw/parent/child.txt",
        "delete_files_from_stdin/in/raw/first/second/third/pna.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
