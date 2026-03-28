#![cfg(not(target_family = "wasm"))]
use crate::utils::{EmbedExt, TestResources, archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::collections::HashSet;

#[test]
fn create_with_files_from_stdin() {
    setup();
    TestResources::extract_in("raw/", "create_with_files_from_stdin/src/").unwrap();

    let list = [
        "create_with_files_from_stdin/src/raw/empty.txt",
        "create_with_files_from_stdin/src/raw/text.txt",
    ]
    .join("\n");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(list);
    cmd.args([
        "--quiet",
        "c",
        "create_with_files_from_stdin/create_with_files_from_stdin.pna",
        "--overwrite",
        "--files-from-stdin",
        "--unstable",
    ]);
    cmd.assert().success();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "create_with_files_from_stdin/create_with_files_from_stdin.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "create_with_files_from_stdin/src/raw/empty.txt",
        "create_with_files_from_stdin/src/raw/text.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
