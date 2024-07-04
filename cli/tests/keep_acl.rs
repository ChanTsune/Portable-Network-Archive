#![cfg(feature = "acl")]
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_keep_acl() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/keep_all.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/keep_all.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/keep_acl/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
}
