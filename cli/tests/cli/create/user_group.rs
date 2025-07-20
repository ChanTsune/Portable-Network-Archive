#![cfg(any(unix, windows))]
use crate::utils::{archive, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn archive_create_uname_gname() {
    setup();
    TestResources::extract_in("raw/", "archive_create_uname_gname/in/").unwrap();
    cli::Cli::try_parse_from([
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
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive::for_each_entry(
        "archive_create_uname_gname/create_uname_gname.pna",
        |entry| {
            let permission = entry.metadata().permission().unwrap();
            assert_eq!(permission.uname(), "test_user");
            assert_eq!(permission.gname(), "test_group");
        },
    )
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_create_uname_gname/create_uname_gname.pna",
        "--overwrite",
        "--out-dir",
        "archive_create_uname_gname/out/",
        #[cfg(not(windows))]
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
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
    cli::Cli::try_parse_from([
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
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive::for_each_entry("archive_create_uid_gid/create_uid_gid.pna", |entry| {
        let permission = entry.metadata().permission().unwrap();
        assert_eq!(permission.uid(), 0);
        assert_eq!(permission.gid(), 2);
    })
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_create_uid_gid/create_uid_gid.pna",
        "--overwrite",
        "--out-dir",
        "archive_create_uid_gid/out/",
        #[cfg(not(windows))]
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("archive_create_uid_gid/in/", "archive_create_uid_gid/out/").unwrap();
}
