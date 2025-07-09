use crate::utils::{diff::diff, setup, TestResources};
use std::fs;

#[test]
fn create_extract_with_cd() {
    setup();
    TestResources::extract_in("raw/", "create_extract_with_cd/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "create_extract_with_cd/create_extract_with_cd.pna",
        "--overwrite",
        "-C",
        "create_extract_with_cd/in/",
        ".",
    ]);
    cmd.assert().success();

    assert!(fs::exists("create_extract_with_cd/create_extract_with_cd.pna").unwrap());

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        "create_extract_with_cd/create_extract_with_cd.pna",
        "--overwrite",
        "-C",
        ".",
        "--out-dir",
        "create_extract_with_cd/out/",
    ]);
    cmd.assert().success();

    // check completely extracted
    diff("create_extract_with_cd/in/", "create_extract_with_cd/out/").unwrap();
}

#[test]
fn append_with_cd() {
    setup();
    TestResources::extract_in("raw/", "append_with_cd/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "append_with_cd/append_with_cd.pna",
        "--overwrite",
        "-C",
        "append_with_cd/in/",
        ".",
    ]);
    cmd.assert().success();

    assert!(fs::exists("append_with_cd/append_with_cd.pna").unwrap());

    // Copy extra input
    TestResources::extract_in("store.pna", "append_with_cd/in/").unwrap();
    TestResources::extract_in("zstd.pna", "append_with_cd/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "append",
        "append_with_cd/append_with_cd.pna",
        "-C",
        "append_with_cd/in/",
        "store.pna",
        "zstd.pna",
    ]);
    cmd.assert().success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        "append_with_cd/append_with_cd.pna",
        "--overwrite",
        "-C",
        ".",
        "--out-dir",
        "append_with_cd/out/",
    ]);
    cmd.assert().success();

    // check completely extracted
    diff("append_with_cd/in/", "append_with_cd/out/").unwrap();
}

#[test]
fn update_with_cd() {
    setup();
    TestResources::extract_in("raw/", "update_with_cd/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "update_with_cd/update_with_cd.pna",
        "--overwrite",
        "-C",
        "update_with_cd/in/",
        ".",
    ]);
    cmd.assert().success();

    assert!(fs::exists("update_with_cd/update_with_cd.pna").unwrap());

    // Copy extra input
    TestResources::extract_in("store.pna", "update_with_cd/in/").unwrap();
    TestResources::extract_in("zstd.pna", "update_with_cd/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "update",
        "update_with_cd/update_with_cd.pna",
        "-C",
        "update_with_cd/in/",
        ".",
    ]);
    cmd.assert().success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        "update_with_cd/update_with_cd.pna",
        "--overwrite",
        "-C",
        ".",
        "--out-dir",
        "update_with_cd/out/",
    ]);
    cmd.assert().success();

    // check completely extracted
    diff("update_with_cd/in/", "update_with_cd/out/").unwrap();
}

#[test]
fn stdio_with_cd() {
    setup();
    TestResources::extract_in("raw/", "stdio_with_cd/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "-c",
        "-C",
        "stdio_with_cd/in/",
        ".",
    ]);
    let assert = cmd.assert().success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.write_stdin(assert.get_output().stdout.as_slice());
    cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "-x",
        "--overwrite",
        "-C",
        ".",
        "--out-dir",
        "stdio_with_cd/out/",
    ]);
    cmd.assert().success();

    diff("stdio_with_cd/in/", "stdio_with_cd/out/").unwrap();
}
