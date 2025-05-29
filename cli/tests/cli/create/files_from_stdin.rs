#![cfg(not(target_family = "wasm"))]
use crate::utils::{self, diff::diff, setup, TestResources};
use assert_cmd::Command;

#[test]
fn create_with_files_from_stdin() {
    setup();
    TestResources::extract_in("raw/", "create_with_files_from_stdin/src/").unwrap();

    let list = [
        "create_with_files_from_stdin/src/raw/empty.txt",
        "create_with_files_from_stdin/src/raw/text.txt",
    ]
    .join("\n");

    let mut cmd = Command::cargo_bin("pna").unwrap();
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

    let mut cmd = Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        "create_with_files_from_stdin/create_with_files_from_stdin.pna",
        "--overwrite",
        "--out-dir",
        "create_with_files_from_stdin/out/",
        "--strip-components",
        "2",
    ]);
    cmd.assert().success();

    utils::copy_dir_all(
        "create_with_files_from_stdin/src/",
        "create_with_files_from_stdin/expected/",
    )
    .unwrap();
    let to_remove = [
        "create_with_files_from_stdin/expected/raw/first/second/third/pna.txt",
        "create_with_files_from_stdin/expected/raw/parent/child.txt",
        "create_with_files_from_stdin/expected/raw/images/icon.bmp",
        "create_with_files_from_stdin/expected/raw/images/icon.png",
        "create_with_files_from_stdin/expected/raw/images/icon.svg",
        "create_with_files_from_stdin/expected/raw/pna/empty.pna",
        "create_with_files_from_stdin/expected/raw/pna/nest.pna",
    ];
    for file in to_remove {
        utils::remove_with_empty_parents(file).unwrap();
    }

    diff(
        "create_with_files_from_stdin/expected/",
        "create_with_files_from_stdin/out/",
    )
    .unwrap();
}
