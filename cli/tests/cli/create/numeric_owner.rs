#![cfg(any(unix, windows))]
use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

#[test]
fn create_numeric_owner() {
    setup();
    TestResources::extract_in("raw/", "numeric_owner/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "numeric_owner/numeric_owner.pna",
        "--overwrite",
        "numeric_owner/in/",
        "--keep-permission",
        "--numeric-owner",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive::for_each_entry("numeric_owner/numeric_owner.pna", |entry| {
        let m = entry.metadata();
        assert!(m.owner_user_name().is_none());
        assert!(m.owner_group_name().is_none());
        #[cfg(unix)]
        {
            assert!(m.owner_uid().is_some());
            assert!(m.owner_gid().is_some());
        }
        #[cfg(windows)]
        {
            assert!(m.owner_uid().is_none());
            assert!(m.owner_gid().is_none());
        }
        assert!(m.permission_mode().is_some());
    })
    .unwrap();
}

#[cfg(windows)]
#[test]
fn create_numeric_owner_with_explicit_ids() {
    setup();
    TestResources::extract_in("raw/", "numeric_owner_with_ids/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "numeric_owner_with_ids/numeric_owner.pna",
        "--overwrite",
        "numeric_owner_with_ids/in/",
        "--keep-permission",
        "--numeric-owner",
        "--uid",
        "1001",
        "--gid",
        "1002",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive::for_each_entry("numeric_owner_with_ids/numeric_owner.pna", |entry| {
        let m = entry.metadata();
        assert!(m.owner_user_name().is_none());
        assert!(m.owner_group_name().is_none());
        assert_eq!(m.owner_uid().map(|id| id.get()), Some(1001));
        assert_eq!(m.owner_gid().map(|id| id.get()), Some(1002));
        assert!(m.permission_mode().is_some());
    })
    .unwrap();
}
