use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;

#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "list_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "list_missing/archive.pna",
        "--overwrite",
        "list_missing/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "list",
        "list_missing/archive.pna",
        "list_missing/in/raw/empty.txt",
        "list_missing/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}
