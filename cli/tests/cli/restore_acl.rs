#![cfg(feature = "acl")]
use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn extract_windows_acl() {
    setup();
    TestResources::extract_in("windows_acl.pna", env!("CARGO_TARGET_TMPDIR")).unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/windows_acl.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/windows_acl/out/"),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_linux_acl() {
    setup();
    TestResources::extract_in("linux_acl.pna", env!("CARGO_TARGET_TMPDIR")).unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/linux_acl.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/linux_acl/out/"),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_macos_acl() {
    setup();
    TestResources::extract_in("macos_acl.pna", env!("CARGO_TARGET_TMPDIR")).unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/macos_acl.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/macos_acl/out/"),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_freebsd_acl() {
    setup();
    TestResources::extract_in("freebsd_acl.pna", env!("CARGO_TARGET_TMPDIR")).unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/freebsd_acl.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/freebsd_acl/out/"),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn extract_generic_acl() {
    setup();
    TestResources::extract_in("generic_acl.pna", ".").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "generic_acl.pna",
        "--overwrite",
        "--out-dir",
        "generic_acl/out/",
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}
