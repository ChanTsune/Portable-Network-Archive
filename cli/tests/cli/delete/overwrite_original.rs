use crate::utils::{archive, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries from the archive
///         by `pna experimental delete`.
/// Expectation: Removes all entries that match the given patterns from the archive
///              and overwrites the original archive file with the result.
#[test]
fn delete_overwrite() {
    setup();
    TestResources::extract_in("raw/", "delete_overwrite/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_overwrite/delete_overwrite.pna",
        "--overwrite",
        "delete_overwrite/in/",
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
        "delete_overwrite/delete_overwrite.pna",
        "**/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("delete_overwrite/delete_overwrite.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "delete_overwrite/in/raw/images/icon.bmp",
        "delete_overwrite/in/raw/pna/empty.pna",
        "delete_overwrite/in/raw/images/icon.svg",
        "delete_overwrite/in/raw/text.txt",
        "delete_overwrite/in/raw/parent/child.txt",
        "delete_overwrite/in/raw/images/icon.png",
        "delete_overwrite/in/raw/pna/nest.pna",
        "delete_overwrite/in/raw/first/second/third/pna.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
