#![cfg(feature = "acl")]
use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_keep_acl() {
    setup();
    TestResources::extract_in("raw/", "keep_acl/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "keep_acl/keep_acl.pna",
        "--overwrite",
        "keep_acl/in/",
        "--keep-acl",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
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
        &components_count("keep_acl/in/").to_string(),
    ]))
    .unwrap();

    diff("keep_acl/in/", "keep_acl/out/").unwrap();
}
