#![cfg(feature = "acl")]
use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn archive_keep_acl() {
    setup();
    TestResources::extract_in("raw/", "keep_acl/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "keep_acl/keep_acl.pna",
        "--overwrite",
        "keep_acl/in/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "keep_acl/keep_acl.pna",
        "--overwrite",
        "--out-dir",
        "keep_acl/out/",
        "--keep-acl",
        "--unstable",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("keep_acl/in/", "keep_acl/out/").unwrap();
}
