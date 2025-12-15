use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;

/// Precondition: The source tree is archived and split into multiple parts.
/// Action: Run `pna experimental delete` on the multipart archive to remove a specific entry.
/// Expectation: The specified entry is removed from the resulting archive while other entries remain.
#[test]
fn delete_from_multipart_archive() {
    setup();
    TestResources::extract_in("raw/", "delete_multipart/in/").unwrap();

    // Create a regular archive first
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "delete_multipart/archive.pna",
        "--overwrite",
        "delete_multipart/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Split the archive into multiple parts
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "delete_multipart/archive.pna",
        "--overwrite",
        "--max-size",
        "1kb",
        "--out-dir",
        "delete_multipart/split/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Delete an entry from the multipart archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_multipart/split/archive.part1.pna",
        "**/raw/text.txt",
        "--output",
        "delete_multipart/deleted.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("delete_multipart/deleted.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Verify text.txt was deleted
    assert!(
        !seen.contains("delete_multipart/in/raw/text.txt"),
        "text.txt should have been deleted"
    );

    // Verify other entries remain
    for required in [
        "delete_multipart/in/raw/empty.txt",
        "delete_multipart/in/raw/images/icon.png",
        "delete_multipart/in/raw/images/icon.bmp",
        "delete_multipart/in/raw/images/icon.svg",
        "delete_multipart/in/raw/pna/empty.pna",
        "delete_multipart/in/raw/pna/nest.pna",
        "delete_multipart/in/raw/parent/child.txt",
        "delete_multipart/in/raw/first/second/third/pna.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
