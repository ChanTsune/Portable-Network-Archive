#![cfg(unix)]
use crate::utils::{diff::diff, setup, TestResources};
use std::fs;

#[test]
fn archive_extract_chroot() {
    // chroot need root privileges
    if !nix::unistd::Uid::effective().is_root() {
        return;
    }
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/extract_chroot/in/"),
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/extract_chroot/extract_chroot.pna"
        ),
        "--overwrite",
        "-C",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/extract_chroot/in/"),
        ".",
    ]);
    cmd.assert().success();

    assert!(fs::exists(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/extract_chroot/extract_chroot.pna"
    ))
    .unwrap());

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/extract_chroot/extract_chroot.pna"
        ),
        "--overwrite",
        "-C",
        env!("CARGO_TARGET_TMPDIR"),
        "--chroot",
        "--out-dir",
        "/extract_chroot/out/",
    ]);
    cmd.assert().success();

    // check completely extracted
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/extract_chroot/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/extract_chroot/out/"),
    )
    .unwrap();
}
