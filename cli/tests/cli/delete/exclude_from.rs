use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn delete_with_exclude_from() {
    setup();
    TestResources::extract_in("raw/", "delete_exclude_from/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_exclude_from/exclude_from.pna",
        "--overwrite",
        "delete_exclude_from/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let file_path = "delete_exclude_from/exclude_list";
    fs::write(file_path, "**/raw/text.txt").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_exclude_from/exclude_from.pna",
        "**/*.txt",
        "--exclude-from",
        file_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::remove_file("delete_exclude_from/in/raw/empty.txt").unwrap();
    fs::remove_file("delete_exclude_from/in/raw/parent/child.txt").unwrap();
    fs::remove_file("delete_exclude_from/in/raw/first/second/third/pna.txt").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_exclude_from/exclude_from.pna",
        "--overwrite",
        "--out-dir",
        "delete_exclude_from/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("delete_exclude_from/in/", "delete_exclude_from/out/").unwrap();
}
