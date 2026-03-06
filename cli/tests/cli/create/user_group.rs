#![cfg(any(unix, windows))]
use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

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

    assert_eq!(
        fs::read("archive_create_uname_gname/out/raw/text.txt").unwrap(),
        fs::read("archive_create_uname_gname/in/raw/text.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uname_gname/out/raw/empty.txt").unwrap(),
        fs::read("archive_create_uname_gname/in/raw/empty.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uname_gname/out/raw/first/second/third/pna.txt").unwrap(),
        fs::read("archive_create_uname_gname/in/raw/first/second/third/pna.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uname_gname/out/raw/images/icon.bmp").unwrap(),
        fs::read("archive_create_uname_gname/in/raw/images/icon.bmp").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uname_gname/out/raw/images/icon.png").unwrap(),
        fs::read("archive_create_uname_gname/in/raw/images/icon.png").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uname_gname/out/raw/images/icon.svg").unwrap(),
        fs::read("archive_create_uname_gname/in/raw/images/icon.svg").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uname_gname/out/raw/parent/child.txt").unwrap(),
        fs::read("archive_create_uname_gname/in/raw/parent/child.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uname_gname/out/raw/pna/empty.pna").unwrap(),
        fs::read("archive_create_uname_gname/in/raw/pna/empty.pna").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uname_gname/out/raw/pna/nest.pna").unwrap(),
        fs::read("archive_create_uname_gname/in/raw/pna/nest.pna").unwrap(),
    );
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

    assert_eq!(
        fs::read("archive_create_uid_gid/out/raw/text.txt").unwrap(),
        fs::read("archive_create_uid_gid/in/raw/text.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uid_gid/out/raw/empty.txt").unwrap(),
        fs::read("archive_create_uid_gid/in/raw/empty.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uid_gid/out/raw/first/second/third/pna.txt").unwrap(),
        fs::read("archive_create_uid_gid/in/raw/first/second/third/pna.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uid_gid/out/raw/images/icon.bmp").unwrap(),
        fs::read("archive_create_uid_gid/in/raw/images/icon.bmp").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uid_gid/out/raw/images/icon.png").unwrap(),
        fs::read("archive_create_uid_gid/in/raw/images/icon.png").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uid_gid/out/raw/images/icon.svg").unwrap(),
        fs::read("archive_create_uid_gid/in/raw/images/icon.svg").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uid_gid/out/raw/parent/child.txt").unwrap(),
        fs::read("archive_create_uid_gid/in/raw/parent/child.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uid_gid/out/raw/pna/empty.pna").unwrap(),
        fs::read("archive_create_uid_gid/in/raw/pna/empty.pna").unwrap(),
    );
    assert_eq!(
        fs::read("archive_create_uid_gid/out/raw/pna/nest.pna").unwrap(),
        fs::read("archive_create_uid_gid/in/raw/pna/nest.pna").unwrap(),
    );
}
