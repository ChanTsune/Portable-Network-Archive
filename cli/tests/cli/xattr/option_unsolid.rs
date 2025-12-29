use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;

/// Precondition: A solid archive contains entries.
/// Action: Run `pna xattr set` with `--unsolid` to set an extended attribute.
/// Expectation: The xattr is applied and the archive is converted to non-solid mode.
#[test]
fn xattr_set_unsolid() {
    setup();
    TestResources::extract_in("raw/", "xattr_unsolid/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_unsolid/archive.pna",
        "--overwrite",
        "--solid",
        "xattr_unsolid/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive is solid before operation
    let mut archive = pna::Archive::open("xattr_unsolid/archive.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, pna::ReadEntry::Solid(_))),
        "archive should be solid before operation"
    );
    drop((archive, entries)); // Release file handle before modifying (required on Windows)

    // Set xattr with --unsolid
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_unsolid/archive.pna",
        "--unsolid",
        "--name",
        "user.author",
        "--value",
        "pna developers",
        "xattr_unsolid/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify xattr was applied
    archive::for_each_entry("xattr_unsolid/archive.pna", |entry| {
        if entry.name() == "xattr_unsolid/in/raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 1, "entry should have exactly one xattr");
            assert_eq!(xattrs[0].name(), "user.author");
            assert_eq!(xattrs[0].value(), b"pna developers");
        }
    })
    .unwrap();

    // Verify archive is now non-solid
    let mut archive = pna::Archive::open("xattr_unsolid/archive.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, pna::ReadEntry::Normal(_))),
        "archive should be non-solid after --unsolid operation"
    );
}

/// Precondition: A solid archive contains multiple entries.
/// Action: Run `pna xattr set` with `--unsolid` to set xattr on one entry.
/// Expectation: All entries become non-solid and the xattr is applied to the target entry only.
#[test]
fn xattr_set_unsolid_multiple_entries() {
    setup();
    TestResources::extract_in("raw/", "xattr_unsolid_multi/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_unsolid_multi/archive.pna",
        "--overwrite",
        "--solid",
        "xattr_unsolid_multi/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set xattr with --unsolid on one specific entry
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_unsolid_multi/archive.pna",
        "--unsolid",
        "--name",
        "user.marker",
        "--value",
        "marked",
        "xattr_unsolid_multi/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify xattr applied only to target entry
    let mut entry_count = 0;
    archive::for_each_entry("xattr_unsolid_multi/archive.pna", |entry| {
        entry_count += 1;
        if entry.name() == "xattr_unsolid_multi/in/raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 1, "target entry should have xattr");
            assert_eq!(xattrs[0].name(), "user.marker");
        } else {
            assert!(
                entry.xattrs().is_empty(),
                "other entries should have no xattrs"
            );
        }
    })
    .unwrap();
    assert!(entry_count > 1, "archive should contain multiple entries");

    // Verify all entries are now non-solid
    let mut archive = pna::Archive::open("xattr_unsolid_multi/archive.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, pna::ReadEntry::Normal(_))),
        "all entries should be non-solid after --unsolid operation"
    );
    assert_eq!(
        entries.len(),
        entry_count,
        "entry count should match after unsolid"
    );
}
