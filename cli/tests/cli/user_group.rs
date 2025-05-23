#![cfg(unix)]
use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_create_uname_gname() {
    setup();
    TestResources::extract_in("raw/", "archive_create_uname_gname/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_create_uname_gname/create_uname_gname.pna",
        "--overwrite",
        "archive_create_uname_gname/in/",
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
        "archive_create_uname_gname/create_uname_gname.pna",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_create_uname_gname/create_uname_gname.pna",
        "--overwrite",
        "--out-dir",
        "archive_create_uname_gname/out/",
        "--keep-permission",
        "--strip-components",
        &components_count("archive_create_uname_gname/in/").to_string(),
    ]))
    .unwrap();

    diff(
        "archive_create_uname_gname/in/",
        "archive_create_uname_gname/out/",
    )
    .unwrap();
}

#[test]
fn archive_create_uid_gid() {
    setup();
    TestResources::extract_in("raw/", "archive_create_uid_gid/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_create_uid_gid/create_uid_gid.pna",
        "--overwrite",
        "archive_create_uid_gid/in/",
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
        "archive_create_uid_gid/create_uid_gid.pna",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_create_uid_gid/create_uid_gid.pna",
        "--overwrite",
        "--out-dir",
        "archive_create_uid_gid/out/",
        "--keep-permission",
        "--strip-components",
        &components_count("archive_create_uid_gid/in/").to_string(),
    ]))
    .unwrap();

    diff("archive_create_uid_gid/in/", "archive_create_uid_gid/out/").unwrap();
}
