use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;
use std::collections::HashSet;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` with `--solid` to build a solid mode archive, then delete entries from
///         the archive by `pna experimental delete` with `--unsolid`.
/// Expectation: Removes all entries that match the given patterns from the archive
///              and overwrites the original archive file with a result that is no longer in solid mode.
#[test]
fn delete_with_unsolid() {
    setup();
    TestResources::extract_in("raw/", "delete_with_unsolid/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_with_unsolid/delete_with_unsolid.pna",
        "--overwrite",
        "--solid",
        "delete_with_unsolid/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "--unsolid",
        "-f",
        "delete_with_unsolid/delete_with_unsolid.pna",
        "**/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("delete_with_unsolid/delete_with_unsolid.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    let mut archive = pna::Archive::open("delete_with_unsolid/delete_with_unsolid.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();

    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, pna::ReadEntry::Normal(_)))
    );
    assert_eq!(entries.len(), 8);

    for required in [
        "delete_with_unsolid/in/raw/empty.txt",
        "delete_with_unsolid/in/raw/parent/child.txt",
        "delete_with_unsolid/in/raw/pna/empty.pna",
        "delete_with_unsolid/in/raw/pna/nest.pna",
        "delete_with_unsolid/in/raw/images/icon.png",
        "delete_with_unsolid/in/raw/images/icon.bmp",
        "delete_with_unsolid/in/raw/first/second/third/pna.txt",
        "delete_with_unsolid/in/raw/images/icon.svg",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
