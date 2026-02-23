use crate::utils::{EmbedExt, TestResources, archive, diff::diff, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;
use std::path::Path;

#[test]
fn archive_keep_all() {
    setup();
    TestResources::extract_in("raw/", "archive_keep_all/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_keep_all/keep_all.pna",
        "--overwrite",
        "archive_keep_all/in/",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("archive_keep_all/keep_all.pna").unwrap());
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_keep_all/keep_all.pna",
        "--overwrite",
        "--out-dir",
        "archive_keep_all/out/",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--strip-components",
        "2",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_paths = std::collections::HashSet::new();
    archive::for_each_entry("archive_keep_all/keep_all.pna", |entry| {
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

    diff("archive_keep_all/in/", "archive_keep_all/out/").unwrap();

    assert_eq!(
        fs::read("archive_keep_all/out/raw/text.txt").unwrap(),
        fs::read("archive_keep_all/in/raw/text.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_keep_all/out/raw/empty.txt").unwrap(),
        fs::read("archive_keep_all/in/raw/empty.txt").unwrap(),
    );
    assert_eq!(
        fs::read("archive_keep_all/out/raw/images/icon.png").unwrap(),
        fs::read("archive_keep_all/in/raw/images/icon.png").unwrap(),
    );
    assert!(Path::new("archive_keep_all/out/raw").is_dir());
    assert!(Path::new("archive_keep_all/out/raw/images").is_dir());
    assert!(Path::new("archive_keep_all/out/raw/text.txt").is_file());
    assert!(Path::new("archive_keep_all/out/raw/empty.txt").is_file());
    assert!(Path::new("archive_keep_all/out/raw/images/icon.png").is_file());
}
