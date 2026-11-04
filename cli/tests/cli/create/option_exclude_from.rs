use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

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
        "-f",
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
        "-f",
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

    // Excluded .txt files should not be in output
    assert!(!fs::exists("create_with_exclude_from/out/raw/text.txt").unwrap());
    assert!(!fs::exists("create_with_exclude_from/out/raw/empty.txt").unwrap());
    assert!(!fs::exists("create_with_exclude_from/out/raw/first/second/third/pna.txt").unwrap());
    assert!(!fs::exists("create_with_exclude_from/out/raw/parent/child.txt").unwrap());
    // Non-excluded files should be present
    assert!(fs::exists("create_with_exclude_from/out/raw/images/icon.png").unwrap());
}
