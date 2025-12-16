use crate::utils::{EmbedExt, TestResources, diff::diff, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

#[test]
fn archive_update_deletion() {
    setup();
    TestResources::extract_in("raw/", "archive_update_deletion/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_update_deletion/update_deletion.pna",
        "--overwrite",
        "archive_update_deletion/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::remove_file("archive_update_deletion/in/raw/empty.txt").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "archive_update_deletion/update_deletion.pna",
        "archive_update_deletion/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_update_deletion/update_deletion.pna",
        "--overwrite",
        "--out-dir",
        "archive_update_deletion/out/",
        "--keep-timestamp",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // restore original empty.txt
    TestResources::extract_in("raw/empty.txt", "archive_update_deletion/in/").unwrap();

    diff(
        "archive_update_deletion/in/",
        "archive_update_deletion/out/",
    )
    .unwrap();
}
