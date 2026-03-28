use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

#[test]
fn extract_with_exclude() {
    setup();
    TestResources::extract_in("raw/", "extract_with_exclude/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_with_exclude/extract_with_exclude.pna",
        "--overwrite",
        "extract_with_exclude/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("extract_with_exclude/extract_with_exclude.pna").unwrap());

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_with_exclude/extract_with_exclude.pna",
        "--overwrite",
        "--out-dir",
        "extract_with_exclude/out/",
        "--strip-components",
        "2",
        "--exclude",
        "**.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Excluded .txt files should not be in output
    assert!(!fs::exists("extract_with_exclude/out/raw/text.txt").unwrap());
    assert!(!fs::exists("extract_with_exclude/out/raw/empty.txt").unwrap());
    assert!(!fs::exists("extract_with_exclude/out/raw/first/second/third/pna.txt").unwrap());
    assert!(!fs::exists("extract_with_exclude/out/raw/parent/child.txt").unwrap());
    // Non-excluded files should be present
    assert!(fs::exists("extract_with_exclude/out/raw/images/icon.png").unwrap());
}
