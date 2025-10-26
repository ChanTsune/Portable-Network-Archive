#![cfg(feature = "acl")]
use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

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
}

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
}

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
}

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
}

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
}
