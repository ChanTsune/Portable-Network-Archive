use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_acl_get_set() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/acl_get_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        &format!("{}/acl_get_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
        "-m",
        "u:test:r,w,x",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        &format!("{}/acl_get_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
        "-m",
        "g:test_group:r,w,x",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        &format!("{}/acl_get_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
        "-x",
        "g:test_group",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "get",
        &format!("{}/acl_get_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/acl_get_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/acl_get_set/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}
