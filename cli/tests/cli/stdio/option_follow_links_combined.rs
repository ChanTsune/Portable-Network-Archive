#![cfg(not(target_family = "wasm"))]
#![cfg(unix)]
#![allow(non_snake_case)]

use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::{fs, os::unix::fs::symlink, path::Path};

fn make_chain_fixture(dir: impl AsRef<Path>) {
    let dir = dir.as_ref();
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("chain_final"), b"final-content").unwrap();
    symlink("chain_final", dir.join("chain_b")).unwrap();
    symlink("chain_b", dir.join("target")).unwrap();
}

fn list_archive(archive: &str) -> String {
    let out = cargo_bin_cmd!("pna")
        .args(["compat", "bsdtar", "--unstable", "-tvf", archive])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(out).unwrap()
}

/// Precondition: source dir contains a chain symlink: target -> chain_b -> chain_final (file).
/// Action: pna compat bsdtar -cLHf (parse order: -L then -H). bsdtar resolves these flags by
///   last-wins, so the effective mode is -H (follow only command-line symlinks).
/// Expectation: nested symlinks (chain_b, target) are preserved as symlinks in the archive.
#[test]
fn stdio_create_with_L_then_H_uses_H_semantics() {
    setup();
    let src = "stdio_LH_then/src";
    let archive = "stdio_LH_then/out.tar";
    make_chain_fixture(src);

    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "--unstable",
            "-cLHf",
            archive,
            "-C",
            src,
            ".",
        ])
        .assert()
        .success();

    let listing = list_archive(archive);
    assert!(
        listing.contains("chain_b -> chain_final"),
        "expected chain_b symlink preserved; listing:\n{listing}"
    );
    assert!(
        listing.contains("target -> chain_b"),
        "expected target symlink preserved; listing:\n{listing}"
    );
}

/// Precondition: same chain fixture as above.
/// Action: pna compat bsdtar -cHLf (parse order: -H then -L). Last-wins makes the effective
///   mode -L (follow all symlinks).
/// Expectation: all symlinks are dereferenced; archive contains regular files only.
#[test]
fn stdio_create_with_H_then_L_uses_L_semantics() {
    setup();
    let src = "stdio_HL_then/src";
    let archive = "stdio_HL_then/out.tar";
    make_chain_fixture(src);

    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "--unstable",
            "-cHLf",
            archive,
            "-C",
            src,
            ".",
        ])
        .assert()
        .success();

    let listing = list_archive(archive);
    assert!(
        !listing.contains(" -> "),
        "expected no symlinks in archive (all dereferenced); listing:\n{listing}"
    );
}

/// Precondition: same chain fixture.
/// Action: pna compat bsdtar -cLf (only -L specified).
/// Expectation: all symlinks are dereferenced.
#[test]
fn stdio_create_with_L_only_dereferences_all_symlinks() {
    setup();
    let src = "stdio_L_only/src";
    let archive = "stdio_L_only/out.tar";
    make_chain_fixture(src);

    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "--unstable",
            "-cLf",
            archive,
            "-C",
            src,
            ".",
        ])
        .assert()
        .success();

    let listing = list_archive(archive);
    assert!(
        !listing.contains(" -> "),
        "expected no symlinks in archive (all dereferenced); listing:\n{listing}"
    );
}

/// Precondition: same chain fixture.
/// Action: pna compat bsdtar -cHf (only -H, command-line symlinks only).
/// Expectation: nested symlinks under "." are preserved (the operand "." is not a symlink itself,
///   so -H does not dereference any chain entry).
#[test]
fn stdio_create_with_H_only_preserves_nested_symlinks() {
    setup();
    let src = "stdio_H_only/src";
    let archive = "stdio_H_only/out.tar";
    make_chain_fixture(src);

    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "--unstable",
            "-cHf",
            archive,
            "-C",
            src,
            ".",
        ])
        .assert()
        .success();

    let listing = list_archive(archive);
    assert!(
        listing.contains("chain_b -> chain_final"),
        "expected chain_b symlink preserved; listing:\n{listing}"
    );
    assert!(
        listing.contains("target -> chain_b"),
        "expected target symlink preserved; listing:\n{listing}"
    );
}
