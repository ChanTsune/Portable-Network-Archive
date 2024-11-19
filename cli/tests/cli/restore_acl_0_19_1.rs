//! Compatibility test with before 0.19.1 specification ACLs
#![cfg(feature = "acl")]
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn extract_windows_acl() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../resources/test/0.19.1/windows_acl.pna",
        "--overwrite",
        "--out-dir",
        &format!("{}/0.19.1/windows_acl/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_linux_acl() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../resources/test/0.19.1/linux_acl.pna",
        "--overwrite",
        "--out-dir",
        &format!("{}/0.19.1/linux_acl/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_macos_acl() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../resources/test/0.19.1/macos_acl.pna",
        "--overwrite",
        "--out-dir",
        &format!("{}/0.19.1/macos_acl/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_freebsd_acl() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../resources/test/0.19.1/freebsd_acl.pna",
        "--overwrite",
        "--out-dir",
        &format!("{}/0.19.1/freebsd_acl/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}
