use crate::utils::{archive, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;
use std::fs;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries from
///         the archive using `pna experimental delete` with `--exclude-from`.
/// Expectation: Target entries except those excluded by `--exclude-from` are removed.
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
        "-f",
        "delete_exclude_from/exclude_from.pna",
        "**/*.txt",
        "--exclude-from",
        file_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();

    archive::for_each_entry("delete_exclude_from/exclude_from.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "delete_exclude_from/in/raw/images/icon.bmp",
        "delete_exclude_from/in/raw/pna/nest.pna",
        "delete_exclude_from/in/raw/images/icon.png",
        "delete_exclude_from/in/raw/images/icon.svg",
        "delete_exclude_from/in/raw/text.txt", // --exclude-from (file content "**/raw/text.txt")
        "delete_exclude_from/in/raw/pna/empty.pna",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
