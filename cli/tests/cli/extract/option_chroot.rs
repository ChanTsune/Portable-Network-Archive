#![cfg(all(unix, not(target_family = "wasm")))]
use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: Archive contains entries and extraction uses absolute `--out-dir` path with `--chroot`.
/// Action: Extract with `--chroot` option where `-C .` sets the chroot root.
/// Expectation: Absolute output path is resolved relative to chroot root, not filesystem root.
#[test]
fn archive_extract_chroot() {
    // chroot need root privileges
    if !nix::unistd::Uid::effective().is_root() {
        return;
    }
    setup();
    TestResources::extract_in("zstd.pna", "extract_chroot/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "x",
        "extract_chroot/zstd.pna",
        "--overwrite",
        "-C",
        ".",
        "--chroot",
        "--out-dir",
        // Absolute path - with chroot this resolves to ./extract_chroot/out/
        "/extract_chroot/out/",
    ]);
    cmd.assert().success();

    // Verify chroot resolved absolute path within test workspace (not at system root)
    assert!(
        fs::exists("extract_chroot/out/raw/text.txt").unwrap(),
        "Extracted file should exist within chroot-relative path"
    );
    assert!(
        fs::exists("extract_chroot/out/raw/images/icon.png").unwrap(),
        "Nested extracted file should exist within chroot-relative path"
    );
}
