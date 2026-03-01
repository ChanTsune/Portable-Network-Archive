#![cfg(feature = "acl")]
use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

#[test]
fn archive_keep_acl() {
    setup();
    TestResources::extract_in("raw/", "keep_acl/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "keep_acl/keep_acl.pna",
        "--overwrite",
        "keep_acl/in/",
        "--keep-acl",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    let mut entry_paths = std::collections::HashSet::new();
    archive::for_each_entry("keep_acl/keep_acl.pna", |entry| {
        entry_paths.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert!(entry_paths.iter().any(|p| p.ends_with("raw/text.txt")));
    assert!(entry_paths.iter().any(|p| p.ends_with("raw/empty.txt")));
    assert!(
        entry_paths
            .iter()
            .any(|p| p.ends_with("raw/images/icon.png"))
    );
    cli::Cli::try_parse_from([
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
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        fs::read("keep_acl/out/raw/text.txt").unwrap(),
        fs::read("keep_acl/in/raw/text.txt").unwrap(),
    );
    assert_eq!(
        fs::read("keep_acl/out/raw/empty.txt").unwrap(),
        fs::read("keep_acl/in/raw/empty.txt").unwrap(),
    );
    assert_eq!(
        fs::read("keep_acl/out/raw/images/icon.png").unwrap(),
        fs::read("keep_acl/in/raw/images/icon.png").unwrap(),
    );
}
