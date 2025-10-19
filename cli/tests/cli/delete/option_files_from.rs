use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;
use std::fs;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries by
///         `pna experimental delete` with `--files-from`.
/// Expectation: Only the entries listed in the file are removed; all other files remain.
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
        "-f",
        "delete_files_from/delete_files_from.pna",
        "--files-from",
        list_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("delete_files_from/delete_files_from.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "delete_files_from/in/raw/images/icon.svg",
        "delete_files_from/in/raw/images/icon.bmp",
        "delete_files_from/in/raw/pna/empty.pna",
        "delete_files_from/in/raw/pna/nest.pna",
        "delete_files_from/in/raw/images/icon.png",
        "delete_files_from/in/raw/parent/child.txt",
        "delete_files_from/in/raw/first/second/third/pna.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
