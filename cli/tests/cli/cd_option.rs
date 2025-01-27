use crate::utils::{diff::diff, setup, TestResources};
use std::fs;

#[test]
fn create_extract_with_cd() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/create_extract_with_cd/in/"),
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/create_extract_with_cd/create_extract_with_cd.pna"
        ),
        "--overwrite",
        "-C",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/create_extract_with_cd/in/"),
        "-r",
        ".",
    ]);
    cmd.assert().success();

    assert!(fs::exists(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/create_extract_with_cd/create_extract_with_cd.pna"
    ))
    .unwrap());

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/create_extract_with_cd/create_extract_with_cd.pna"
        ),
        "--overwrite",
        "-C",
        env!("CARGO_TARGET_TMPDIR"),
        "--out-dir",
        "./create_extract_with_cd/out/",
    ]);
    cmd.assert().success();

    // check completely extracted
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/create_extract_with_cd/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/create_extract_with_cd/out/"),
    )
    .unwrap();
}

#[test]
fn append_with_cd() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/append_with_cd/in/"),
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/append_with_cd/append_with_cd.pna"
        ),
        "--overwrite",
        "-C",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/append_with_cd/in/"),
        "-r",
        ".",
    ]);
    cmd.assert().success();

    assert!(fs::exists(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/append_with_cd/append_with_cd.pna"
    ))
    .unwrap());

    // Copy extra input
    TestResources::extract_in(
        "store.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/append_with_cd/in/"),
    )
    .unwrap();
    TestResources::extract_in(
        "zstd.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/append_with_cd/in/"),
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "append",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/append_with_cd/append_with_cd.pna"
        ),
        "-C",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/append_with_cd/in/"),
        "store.pna",
        "zstd.pna",
    ]);
    cmd.assert().success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/append_with_cd/append_with_cd.pna"
        ),
        "--overwrite",
        "-C",
        env!("CARGO_TARGET_TMPDIR"),
        "--out-dir",
        "./append_with_cd/out/",
    ]);
    cmd.assert().success();

    // check completely extracted
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/append_with_cd/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/append_with_cd/out/"),
    )
    .unwrap();
}

#[test]
fn update_with_cd() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/update_with_cd/in/"),
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/update_with_cd/update_with_cd.pna"
        ),
        "--overwrite",
        "-C",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/update_with_cd/in/"),
        "-r",
        ".",
    ]);
    cmd.assert().success();

    assert!(fs::exists(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/update_with_cd/update_with_cd.pna"
    ))
    .unwrap());

    // Copy extra input
    TestResources::extract_in(
        "store.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/update_with_cd/in/"),
    )
    .unwrap();
    TestResources::extract_in(
        "zstd.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/update_with_cd/in/"),
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "update",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/update_with_cd/update_with_cd.pna"
        ),
        "-C",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/update_with_cd/in/"),
        "-r",
        ".",
    ]);
    cmd.assert().success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/update_with_cd/update_with_cd.pna"
        ),
        "--overwrite",
        "-C",
        env!("CARGO_TARGET_TMPDIR"),
        "--out-dir",
        "./update_with_cd/out/",
    ]);
    cmd.assert().success();

    // check completely extracted
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/update_with_cd/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/update_with_cd/out/"),
    )
    .unwrap();
}
