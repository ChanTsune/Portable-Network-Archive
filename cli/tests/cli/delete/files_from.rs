use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn delete_with_files_from() {
    setup();
    TestResources::extract_in("raw/", "delete_files_from/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_files_from/delete_files_from.pna",
        "--overwrite",
        "delete_files_from/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let list_path = "delete_files_from/delete_list";
    fs::write(
        list_path,
        ["**/raw/empty.txt", "**/raw/text.txt"].join("\n"),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_files_from/delete_files_from.pna",
        "--files-from",
        list_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::remove_file("delete_files_from/in/raw/empty.txt").unwrap();
    fs::remove_file("delete_files_from/in/raw/text.txt").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_files_from/delete_files_from.pna",
        "--overwrite",
        "--out-dir",
        "delete_files_from/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("delete_files_from/in/", "delete_files_from/out/").unwrap();
}
