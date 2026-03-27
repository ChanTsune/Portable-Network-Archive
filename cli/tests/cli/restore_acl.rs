#![cfg(feature = "acl")]
use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive with Windows ACL metadata exists.
/// Action: Run `pna extract` with `--keep-acl`.
/// Expectation: Extraction succeeds and the entry file is materialized.
#[test]
fn extract_windows_acl() {
    setup();
    TestResources::extract_in("windows_acl.pna", ".").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "windows_acl.pna",
        "--overwrite",
        "--out-dir",
        "windows_acl/out/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("windows_acl/out/windows_acl.txt").unwrap());
}

/// Precondition: An archive with Linux ACL metadata exists.
/// Action: Run `pna extract` with `--keep-acl`.
/// Expectation: Extraction succeeds and the entry file is materialized.
#[test]
fn extract_linux_acl() {
    setup();
    TestResources::extract_in("linux_acl.pna", ".").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "linux_acl.pna",
        "--overwrite",
        "--out-dir",
        "linux_acl/out/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("linux_acl/out/linux_acl.txt").unwrap());
}

/// Precondition: An archive with macOS ACL metadata exists.
/// Action: Run `pna extract` with `--keep-acl`.
/// Expectation: Extraction succeeds and the entry file is materialized.
#[test]
fn extract_macos_acl() {
    setup();
    TestResources::extract_in("macos_acl.pna", ".").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "macos_acl.pna",
        "--overwrite",
        "--out-dir",
        "macos_acl/out/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("macos_acl/out/macos_acl.txt").unwrap());
}

/// Precondition: An archive with FreeBSD ACL metadata exists.
/// Action: Run `pna extract` with `--keep-acl`.
/// Expectation: Extraction succeeds and the entry file is materialized.
#[test]
fn extract_freebsd_acl() {
    setup();
    TestResources::extract_in("freebsd_acl.pna", ".").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "freebsd_acl.pna",
        "--overwrite",
        "--out-dir",
        "freebsd_acl/out/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("freebsd_acl/out/freebsd_acl.txt").unwrap());
}

/// Precondition: An archive with generic (platform-independent) ACL metadata exists.
/// Action: Run `pna extract` with `--keep-acl`.
/// Expectation: Extraction succeeds and the entry file is materialized.
#[test]
fn extract_generic_acl() {
    setup();
    TestResources::extract_in("generic_acl.pna", ".").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "generic_acl.pna",
        "--overwrite",
        "--out-dir",
        "generic_acl/out/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("generic_acl/out/generic_acl.txt").unwrap());
}
