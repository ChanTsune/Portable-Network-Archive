use crate::utils::{copy_dir_all, diff::diff, setup};
use std::fs;

#[test]
fn create_extract_with_cd() {
    setup();
    copy_dir_all(
        "../resources/test/raw/",
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
    copy_dir_all(
        "../resources/test/raw/",
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
    fs::copy(
        "../resources/test/store.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/append_with_cd/in/store.pna"),
    )
    .unwrap();
    fs::copy(
        "../resources/test/zstd.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/append_with_cd/in/zstd.pna"),
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
