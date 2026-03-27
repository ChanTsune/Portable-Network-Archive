//! Compatibility test with before 0.19.1 specification ACLs
#![cfg(feature = "acl")]
use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: A v0.19.1 format archive with Windows ACL metadata exists.
/// Action: Run `pna extract` with `--keep-acl`.
/// Expectation: Extraction succeeds and the entry file is materialized.
#[test]
fn extract_windows_acl() {
    setup();
    TestResources::extract_in("0.19.1/windows_acl.pna", ".").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "0.19.1/windows_acl.pna",
        "--overwrite",
        "--out-dir",
        "0.19.1/windows_acl/out/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("0.19.1/windows_acl/out/windows_acl.txt").unwrap());
}

/// Precondition: A v0.19.1 format archive with Linux ACL metadata exists.
/// Action: Run `pna extract` with `--keep-acl`.
/// Expectation: Extraction succeeds and the entry file is materialized.
#[test]
fn extract_linux_acl() {
    setup();
    TestResources::extract_in("0.19.1/linux_acl.pna", ".").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "0.19.1/linux_acl.pna",
        "--overwrite",
        "--out-dir",
        "0.19.1/linux_acl/out/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("0.19.1/linux_acl/out/linux_acl.txt").unwrap());
}

/// Precondition: A v0.19.1 format archive with macOS ACL metadata exists.
/// Action: Run `pna extract` with `--keep-acl`.
/// Expectation: Extraction succeeds and the entry file is materialized.
#[test]
fn extract_macos_acl() {
    setup();
    TestResources::extract_in("0.19.1/macos_acl.pna", ".").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "0.19.1/macos_acl.pna",
        "--overwrite",
        "--out-dir",
        "0.19.1/macos_acl/out/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("0.19.1/macos_acl/out/macos_acl.txt").unwrap());
}

/// Precondition: A v0.19.1 format archive with FreeBSD ACL metadata exists.
/// Action: Run `pna extract` with `--keep-acl`.
/// Expectation: Extraction succeeds and the entry file is materialized.
#[test]
fn extract_freebsd_acl() {
    setup();
    TestResources::extract_in("0.19.1/freebsd_acl.pna", ".").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "0.19.1/freebsd_acl.pna",
        "--overwrite",
        "--out-dir",
        "0.19.1/freebsd_acl/out/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("0.19.1/freebsd_acl/out/freebsd_acl.txt").unwrap());
}
