#![cfg(unix)]
use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_create_uname_gname() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/create_uname_gname.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-permission",
        "--uname",
        "test_user",
        "--gname",
        "test_group",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "ls",
        "-lh",
        &format!("{}/create_uname_gname.pna", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/create_uname_gname.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/create_uname_gname/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-permission",
    ]))
    .unwrap();
}

#[test]
fn archive_create_uid_gid() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/create_uid_gid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-permission",
        "--uid",
        "0",
        "--gid",
        "2",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "ls",
        "-lh",
        "--numeric-owner",
        &format!("{}/create_uid_gid.pna", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/create_uid_gid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/create_uid_gid/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-permission",
    ]))
    .unwrap();
}
