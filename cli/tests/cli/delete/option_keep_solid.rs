use crate::utils::{archive, setup, EmbedExt, TestResources};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` with `--solid` to build a solid mode archive, then delete entries from
///         the archive by `pna experimental delete` with `--keep-solid`.
/// Expectation: Removes all entries that match the given patterns from the archive
///              and overwrites the original archive file while preserving solid mode.
#[test]
fn delete_with_keep_solid() {
    setup();
    TestResources::extract_in("raw/", "delete_with_keep_solid/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_with_keep_solid/delete_with_keep_solid.pna",
        "--overwrite",
        "--solid",
        "delete_with_keep_solid/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_with_keep_solid/delete_with_keep_solid.pna",
        "--keep-solid",
        "**/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "delete_with_keep_solid/delete_with_keep_solid.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    let mut archive =
        pna::Archive::open("delete_with_keep_solid/delete_with_keep_solid.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();

    assert!(entries
        .iter()
        .all(|entry| matches!(entry, pna::ReadEntry::Solid(_))));
    assert_eq!(entries.len(), 1);

    for required in [
        "delete_with_keep_solid/in/raw/first/second/third/pna.txt",
        "delete_with_keep_solid/in/raw/parent/child.txt",
        "delete_with_keep_solid/in/raw/images/icon.bmp",
        "delete_with_keep_solid/in/raw/images/icon.svg",
        "delete_with_keep_solid/in/raw/pna/nest.pna",
        "delete_with_keep_solid/in/raw/empty.txt",
        "delete_with_keep_solid/in/raw/pna/empty.pna",
        "delete_with_keep_solid/in/raw/images/icon.png",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
