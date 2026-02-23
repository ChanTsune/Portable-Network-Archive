#![cfg(feature = "acl")]
use crate::utils::{EmbedExt, TestResources, archive, diff::diff, setup};
use clap::Parser;
use portable_network_archive::cli;

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
    assert!(!entry_paths.is_empty());
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

    diff("keep_acl/in/", "keep_acl/out/").unwrap();
}
