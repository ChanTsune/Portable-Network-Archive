use crate::utils::{self, EmbedExt, TestResources, diff::diff, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: A file contains exclusion patterns (`**/*.txt`) and a directory contains various files.
/// Action: Run `pna create` with `--exclude-from` pointing to the pattern file.
/// Expectation: Files matching the patterns in the file are excluded.
#[test]
fn create_with_exclude_from() {
    setup();
    TestResources::extract_in("raw/", "create_with_exclude_from/in/").unwrap();
    let file_path = "create_with_exclude_from/exclude_list";
    fs::write(file_path, "**/*.txt").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_exclude_from/exclude_from.pna",
        "--overwrite",
        "create_with_exclude_from/in/",
        "--exclude-from",
        file_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_exclude_from/exclude_from.pna",
        "--overwrite",
        "--out-dir",
        "create_with_exclude_from/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let excluded = [
        "create_with_exclude_from/in/raw/first/second/third/pna.txt",
        "create_with_exclude_from/in/raw/parent/child.txt",
        "create_with_exclude_from/in/raw/empty.txt",
        "create_with_exclude_from/in/raw/text.txt",
    ];
    for file in excluded {
        utils::remove_with_empty_parents(file).unwrap();
    }

    diff(
        "create_with_exclude_from/in/",
        "create_with_exclude_from/out/",
    )
    .unwrap();
}
