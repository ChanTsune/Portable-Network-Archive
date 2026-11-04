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
    let mut entry_names = Vec::new();
    archive::for_each_entry("numeric_owner/numeric_owner.pna", |entry| {
        entry_names.push(entry.header().path().to_string());
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
    assert_expected_entries(entry_names, "numeric_owner/in");
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
    let mut entry_names = Vec::new();
    archive::for_each_entry("numeric_owner_with_ids/numeric_owner.pna", |entry| {
        entry_names.push(entry.header().path().to_string());
        let m = entry.metadata();
        assert!(m.owner_user_name().is_none());
        assert!(m.owner_group_name().is_none());
        assert_eq!(m.owner_uid().map(|id| id.get()), Some(1001));
        assert_eq!(m.owner_gid().map(|id| id.get()), Some(1002));
        assert!(m.permission_mode().is_some());
    })
    .unwrap();
    assert_expected_entries(entry_names, "numeric_owner_with_ids/in");
}

fn assert_expected_entries(mut entry_names: Vec<String>, root: &str) {
    let expected_suffixes = [
        "",
        "/raw",
        "/raw/empty.txt",
        "/raw/first",
        "/raw/first/second",
        "/raw/first/second/third",
        "/raw/first/second/third/pna.txt",
        "/raw/images",
        "/raw/images/icon.bmp",
        "/raw/images/icon.png",
        "/raw/images/icon.svg",
        "/raw/parent",
        "/raw/parent/child.txt",
        "/raw/pna",
        "/raw/pna/empty.pna",
        "/raw/pna/nest.pna",
        "/raw/text.txt",
    ];
    let mut expected: Vec<_> = expected_suffixes
        .into_iter()
        .map(|suffix| format!("{root}{suffix}"))
        .collect();
    entry_names.sort();
    expected.sort();
    assert_eq!(entry_names, expected);
}
