use crate::utils::{setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "extract_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_missing/archive.pna",
        "--overwrite",
        "extract_missing/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_missing/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_missing/out/",
        "extract_missing/in/raw/empty.txt",
        "extract_missing/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}
