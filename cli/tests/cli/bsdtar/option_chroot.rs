#![cfg(all(unix, not(target_family = "wasm")))]
use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: Archive contains entries and compat extraction uses absolute `--out-dir` with `--chroot`.
/// Action: Extract with `pna compat bsdtar --chroot` where `-C .` sets the chroot root.
/// Expectation: Absolute output path is resolved relative to chroot root, not filesystem root.
#[test]
fn compat_bsdtar_extract_chroot() {
    // chroot needs root privileges.
    if !nix::unistd::Uid::effective().is_root() {
        return;
    }
    setup();
    TestResources::extract_in("zstd.pna", "extract_chroot/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--unstable",
        "-x",
        "-f",
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

    // Verify chroot resolved absolute path within the test workspace, not at system root.
    assert!(
        fs::exists("extract_chroot/out/raw/text.txt").unwrap(),
        "Extracted file should exist within chroot-relative path"
    );
    assert!(
        fs::exists("extract_chroot/out/raw/images/icon.png").unwrap(),
        "Nested extracted file should exist within chroot-relative path"
    );
}
