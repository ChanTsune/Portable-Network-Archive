#![cfg(not(target_family = "wasm"))]

use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;

/// Precondition: Source tree contains `a/b/file.txt` to be archived via stdio mode.
/// Action: Run `pna experimental stdio --create --strip-components 1 -f <archive> -C <input> a/b/file.txt`,
///         then extract with `pna experimental stdio --extract --overwrite --out-dir <out> -f <archive>`.
/// Expectation: The first path component is dropped in the archive, yielding `b/file.txt` on
///         extract and no recreated `a/` directory.
#[test]
fn stdio_create_respects_strip_components_on_store() {
    setup();

    let base = Path::new("stdio_strip_components");
    let input = base.join("in");
    fs::create_dir_all(input.join("a/b")).unwrap();
    fs::write(input.join("a/b/file.txt"), b"payload").unwrap();

    let archive = base.join("archive.pna");
    let _ = fs::remove_file(&archive);
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--create",
        "--strip-components",
        "1",
        "-f",
        archive.to_str().unwrap(),
        "-C",
        input.to_str().unwrap(),
        "a/b/file.txt",
    ]);
    create_cmd.assert().success();

    let out_dir = base.join("out");
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--extract",
        "--overwrite",
        "--out-dir",
        out_dir.to_str().unwrap(),
        "-f",
        archive.to_str().unwrap(),
    ]);
    extract_cmd.assert().success();

    assert!(out_dir.join("b").join("file.txt").exists());
    assert!(!out_dir.join("a").exists());
}
