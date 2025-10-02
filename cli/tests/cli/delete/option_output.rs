use crate::utils::{archive, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries from the archive
///         by `pna experimental delete` with `--output`.
/// Expectation: Removes all entries that match the given patterns from the archive
///              and creates a new archive file with the result.
#[test]
fn delete_output() {
    setup();
    TestResources::extract_in("raw/", "delete_output/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_output/delete_output.pna",
        "--overwrite",
        "delete_output/in/",
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
        "delete_output/delete_output.pna",
        "**/raw/text.txt",
        "--output",
        "delete_output/deleted.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("delete_output/delete_output.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    for required in [
        "delete_output/in/raw/empty.txt",
        "delete_output/in/raw/first/second/third/pna.txt",
        "delete_output/in/raw/text.txt",
        "delete_output/in/raw/parent/child.txt",
        "delete_output/in/raw/pna/empty.pna",
        "delete_output/in/raw/pna/nest.pna",
        "delete_output/in/raw/images/icon.svg",
        "delete_output/in/raw/images/icon.png",
        "delete_output/in/raw/images/icon.bmp",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");

    let mut seen = HashSet::new();
    archive::for_each_entry("delete_output/deleted.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    for required in [
        "delete_output/in/raw/images/icon.png",
        "delete_output/in/raw/images/icon.svg",
        "delete_output/in/raw/pna/empty.pna",
        "delete_output/in/raw/parent/child.txt",
        "delete_output/in/raw/pna/nest.pna",
        "delete_output/in/raw/empty.txt",
        "delete_output/in/raw/images/icon.bmp",
        "delete_output/in/raw/first/second/third/pna.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
