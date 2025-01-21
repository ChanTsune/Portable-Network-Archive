#![cfg(feature = "acl")]
use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_keep_acl() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/keep_acl/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/keep_acl/keep_acl.pna"),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/keep_acl/in/"),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/keep_acl/keep_acl.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/keep_acl/out/"),
        "--keep-acl",
        "--unstable",
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/keep_acl/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/keep_acl/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/keep_acl/out/"),
    )
    .unwrap();
}
