use crate::utils::{self, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn create_with_files_from() {
    setup();
    TestResources::extract_in("raw/", "create_with_files_from/src/").unwrap();

    let list_path = "create_with_files_from/files.txt";
    fs::write(
        list_path,
        [
            "create_with_files_from/src/raw/empty.txt",
            "create_with_files_from/src/raw/text.txt",
        ]
        .join("\n"),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_files_from/create_with_files_from.pna",
        "--overwrite",
        "--files-from",
        list_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_files_from/create_with_files_from.pna",
        "--overwrite",
        "--out-dir",
        "create_with_files_from/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    utils::copy_dir_all(
        "create_with_files_from/src/",
        "create_with_files_from/expected/",
    )
    .unwrap();
    let to_remove = [
        "create_with_files_from/expected/raw/first/second/third/pna.txt",
        "create_with_files_from/expected/raw/parent/child.txt",
        "create_with_files_from/expected/raw/images/icon.bmp",
        "create_with_files_from/expected/raw/images/icon.png",
        "create_with_files_from/expected/raw/images/icon.svg",
        "create_with_files_from/expected/raw/pna/empty.pna",
        "create_with_files_from/expected/raw/pna/nest.pna",
    ];
    for file in to_remove {
        utils::remove_with_empty_parents(file).unwrap();
    }

    diff(
        "create_with_files_from/expected/",
        "create_with_files_from/out/",
    )
    .unwrap();
}
