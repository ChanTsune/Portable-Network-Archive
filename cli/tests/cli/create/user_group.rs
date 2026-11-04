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
        "-f",
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
            let m = entry.metadata();
            assert_eq!(m.owner_user_name().unwrap().as_str(), "test_user");
            assert_eq!(m.owner_group_name().unwrap().as_str(), "test_group");
        },
    )
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
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

/// Precondition: Ownership is overridden with a uid and gid that have no
/// corresponding user or group on the system
/// Action: Create an archive with permission preservation
/// Expectation: Creation succeeds, the numeric ids are recorded, and no
/// owner names are recorded
#[test]
#[cfg(unix)]
fn archive_create_uid_gid_unresolved() {
    setup();
    #[cfg(not(target_os = "redox"))]
    let uid = (1_000_000..1_010_000)
        .find(|&id| nix::unistd::User::from_uid(id.into()).unwrap().is_none())
        .expect("no unregistered uid in candidate range");
    #[cfg(not(target_os = "redox"))]
    let gid = (1_000_000..1_010_000)
        .find(|&id| nix::unistd::Group::from_gid(id.into()).unwrap().is_none())
        .expect("no unregistered gid in candidate range");
    #[cfg(target_os = "redox")]
    let (uid, gid) = (1_000_000_u32, 1_000_000_u32);
    let uid_arg = uid.to_string();
    let gid_arg = gid.to_string();
    TestResources::extract_in("raw/", "archive_create_uid_gid_unresolved/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "archive_create_uid_gid_unresolved/create_uid_gid_unresolved.pna",
        "--overwrite",
        "archive_create_uid_gid_unresolved/in/",
        "--keep-permission",
        "--uid",
        &uid_arg,
        "--gid",
        &gid_arg,
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive::for_each_entry(
        "archive_create_uid_gid_unresolved/create_uid_gid_unresolved.pna",
        |entry| {
            let m = entry.metadata();
            assert_eq!(m.owner_uid().unwrap().get(), u64::from(uid));
            assert_eq!(m.owner_gid().unwrap().get(), u64::from(gid));
            assert!(m.owner_user_name().is_none());
            assert!(m.owner_group_name().is_none());
        },
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
        "-f",
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
        let m = entry.metadata();
        assert_eq!(m.owner_uid().unwrap().get(), 0);
        assert_eq!(m.owner_gid().unwrap().get(), 2);
    })
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
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
