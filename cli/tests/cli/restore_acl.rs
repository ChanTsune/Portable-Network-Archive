#![cfg(feature = "acl")]
use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn extract_windows_acl() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../resources/test/windows_acl.pna",
        "--overwrite",
        "--out-dir",
        &format!("{}/windows_acl/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_linux_acl() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../resources/test/linux_acl.pna",
        "--overwrite",
        "--out-dir",
        &format!("{}/linux_acl/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_macos_acl() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../resources/test/macos_acl.pna",
        "--overwrite",
        "--out-dir",
        &format!("{}/macos_acl/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_freebsd_acl() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../resources/test/freebsd_acl.pna",
        "--overwrite",
        "--out-dir",
        &format!("{}/freebsd_acl/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}
