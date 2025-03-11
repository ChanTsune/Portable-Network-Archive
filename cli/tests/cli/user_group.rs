#![cfg(unix)]
use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_create_uname_gname() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uname_gname/in/"
        ),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uname_gname/create_uname_gname.pna"
        ),
        "--overwrite",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uname_gname/in/"
        ),
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
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uname_gname/create_uname_gname.pna"
        ),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uname_gname/create_uname_gname.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uname_gname/out/"
        ),
        "--keep-permission",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uname_gname/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    diff(
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uname_gname/in/"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uname_gname/out/"
        ),
    )
    .unwrap();
}

#[test]
fn archive_create_uid_gid() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_create_uid_gid/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uid_gid/create_uid_gid.pna"
        ),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_create_uid_gid/in/"),
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
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uid_gid/create_uid_gid.pna"
        ),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uid_gid/create_uid_gid.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_create_uid_gid/out/"),
        "--keep-permission",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_create_uid_gid/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_create_uid_gid/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_create_uid_gid/out/"),
    )
    .unwrap();
}
