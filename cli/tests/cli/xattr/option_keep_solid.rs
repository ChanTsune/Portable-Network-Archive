use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;

/// Precondition: A solid archive contains entries.
/// Action: Run `pna xattr set` with `--keep-solid` to set an extended attribute.
/// Expectation: The xattr is applied and the archive remains in solid mode.
#[test]
fn xattr_set_keep_solid() {
    setup();
    TestResources::extract_in("raw/", "xattr_keep_solid/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_keep_solid/archive.pna",
        "--overwrite",
        "--solid",
        "xattr_keep_solid/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive is solid before operation
    let mut archive = pna::Archive::open("xattr_keep_solid/archive.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, pna::ReadEntry::Solid(_))),
        "archive should be solid before operation"
    );
    drop((archive, entries)); // Release file handle before modifying (required on Windows)

    // Set xattr with --keep-solid
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_keep_solid/archive.pna",
        "--keep-solid",
        "--name",
        "user.author",
        "--value",
        "pna developers",
        "xattr_keep_solid/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify xattr was applied
    archive::for_each_entry("xattr_keep_solid/archive.pna", |entry| {
        if entry.header().path().as_str() == "xattr_keep_solid/in/raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 1, "entry should have exactly one xattr");
            assert_eq!(xattrs[0].name(), "user.author");
            assert_eq!(xattrs[0].value(), b"pna developers");
        }
    })
    .unwrap();

    // Verify archive remains solid
    let mut archive = pna::Archive::open("xattr_keep_solid/archive.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, pna::ReadEntry::Solid(_))),
        "archive should remain solid after --keep-solid operation"
    );
    assert_eq!(entries.len(), 1, "should have exactly one solid entry");
}

/// Precondition: A solid archive contains entries with existing xattrs.
/// Action: Run `pna xattr set` with `--keep-solid` to add another xattr.
/// Expectation: Both xattrs exist and the archive remains in solid mode.
#[test]
fn xattr_set_keep_solid_preserves_existing() {
    setup();
    TestResources::extract_in("raw/", "xattr_keep_solid_existing/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_keep_solid_existing/archive.pna",
        "--overwrite",
        "--solid",
        "xattr_keep_solid_existing/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set xattrs with --keep-solid
    for (name, value) in [
        ("user.first", "first_value"),
        ("user.second", "second_value"),
    ] {
        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "xattr",
            "set",
            "xattr_keep_solid_existing/archive.pna",
            "--keep-solid",
            "--name",
            name,
            "--value",
            value,
            "xattr_keep_solid_existing/in/raw/empty.txt",
        ])
        .unwrap()
        .execute()
        .unwrap();
    }

    // Verify both xattrs exist
    archive::for_each_entry("xattr_keep_solid_existing/archive.pna", |entry| {
        if entry.header().path().as_str() == "xattr_keep_solid_existing/in/raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 2, "entry should have two xattrs");
            assert!(
                xattrs
                    .iter()
                    .any(|x| x.name() == "user.first" && x.value() == b"first_value"),
                "first xattr should exist"
            );
            assert!(
                xattrs
                    .iter()
                    .any(|x| x.name() == "user.second" && x.value() == b"second_value"),
                "second xattr should exist"
            );
        }
    })
    .unwrap();

    // Verify archive remains solid
    let mut archive = pna::Archive::open("xattr_keep_solid_existing/archive.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, pna::ReadEntry::Solid(_))),
        "archive should remain solid after multiple --keep-solid operations"
    );
}
