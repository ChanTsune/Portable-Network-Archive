#![cfg(all(unix, not(target_family = "wasm")))]
use crate::utils::{diff::diff, setup, EmbedExt, TestResources};
use std::fs;

#[test]
fn archive_extract_chroot() {
    // chroot need root privileges
    if !nix::unistd::Uid::effective().is_root() {
        return;
    }
    setup();
    TestResources::extract_in("raw/", "extract_chroot/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "extract_chroot/extract_chroot.pna",
        "--overwrite",
        "-C",
        "extract_chroot/in/",
        ".",
    ]);
    cmd.assert().success();

    assert!(fs::exists("extract_chroot/extract_chroot.pna").unwrap());

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        "extract_chroot/extract_chroot.pna",
        "--overwrite",
        "-C",
        ".",
        "--chroot",
        "--out-dir",
        "/extract_chroot/out/",
    ]);
    cmd.assert().success();

    // check completely extracted
    diff("extract_chroot/in/", "extract_chroot/out/").unwrap();
}
