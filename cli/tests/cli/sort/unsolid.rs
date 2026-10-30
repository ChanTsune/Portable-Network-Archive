use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;
use std::io::{Read, Write};

fn layout(path: &str) -> Vec<bool> {
    let mut archive = pna::Archive::open(path).unwrap();
    archive
        .entries()
        .map(|entry| matches!(entry.unwrap(), pna::ReadEntry::Solid(_)))
        .collect()
}

/// Precondition: A solid archive contains entries in non-alphabetical order.
/// Action: Run `pna sort` with `--unsolid`.
/// Expectation: Entries are sorted by name and every entry becomes a normal entry.
#[test]
fn sort_unsolid_solid_archive() {
    setup();
    let path = "sort_unsolid.pna";
    archive::create_solid_archive_with_permissions(
        path,
        &[
            FileEntryDef {
                path: "b.txt",
                content: b"b",
                permission: 0o644,
            },
            FileEntryDef {
                path: "a.txt",
                content: b"a",
                permission: 0o644,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from(["pna", "--quiet", "sort", "--unsolid", "-f", path])
        .unwrap()
        .execute()
        .unwrap();

    assert_eq!(layout(path), [false, false]);
    assert_eq!(archive::get_archive_entry_names(path), ["a.txt", "b.txt"]);
}

/// Precondition: An encrypted solid archive contains entries in non-alphabetical order.
/// Action: Run `pna sort` with `--unsolid` and the correct password.
/// Expectation: Entries are sorted by name, every entry becomes a normal entry, and
///              the contents are readable without a password.
#[test]
fn sort_unsolid_encrypted_solid_archive() {
    setup();
    let path = "sort_unsolid_encrypted.pna";
    archive::create_encrypted_solid_archive_with_permissions(
        path,
        &[
            FileEntryDef {
                path: "b.txt",
                content: b"content b",
                permission: 0o644,
            },
            FileEntryDef {
                path: "a.txt",
                content: b"content a",
                permission: 0o644,
            },
        ],
        "password",
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "--unsolid",
        "-f",
        path,
        "--password",
        "password",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(layout(path), [false, false]);

    let mut entries = Vec::new();
    archive::for_each_entry(path, |entry| {
        let mut content = Vec::new();
        entry
            .reader(pna::ReadOptions::with_password::<&[u8]>(None))
            .unwrap()
            .read_to_end(&mut content)
            .unwrap();
        entries.push((entry.header().path().to_string(), content));
    })
    .unwrap();
    assert_eq!(
        entries,
        [
            ("a.txt".to_string(), b"content a".to_vec()),
            ("b.txt".to_string(), b"content b".to_vec()),
        ]
    );
}

fn stored_file_entry(path: &str) -> pna::NormalEntry {
    let mut builder =
        pna::FileEntryBuilder::new_with_options(path.into(), pna::WriteOptions::store()).unwrap();
    builder.write_all(b"content").unwrap();
    builder.build().unwrap()
}

fn solid_block(paths: &[&str]) -> pna::SolidEntry {
    let mut solid = pna::SolidEntryBuilder::new(pna::WriteOptions::store()).unwrap();
    for path in paths {
        solid.add_entry(stored_file_entry(path)).unwrap();
    }
    solid.build().unwrap()
}

/// Precondition: An archive interleaves normal entries around two solid blocks, with
///              entry names out of order both inside and across the blocks.
/// Action: Run `pna sort` with `--unsolid`.
/// Expectation: Every entry becomes a normal entry and the names are sorted globally
///              across the former block boundaries.
#[test]
fn sort_unsolid_mixed_archive() {
    setup();
    let path = "sort_unsolid_mixed.pna";
    let file = std::fs::File::create(path).unwrap();
    let mut out = pna::Archive::write_header(file).unwrap();
    out.add_entry(stored_file_entry("e.txt")).unwrap();
    out.add_entry(solid_block(&["d.txt", "b.txt"])).unwrap();
    out.add_entry(stored_file_entry("a.txt")).unwrap();
    out.add_entry(solid_block(&["c.txt"])).unwrap();
    out.finalize().unwrap();

    cli::Cli::try_parse_from(["pna", "--quiet", "sort", "--unsolid", "-f", path])
        .unwrap()
        .execute()
        .unwrap();

    assert_eq!(layout(path), [false, false, false, false, false]);
    assert_eq!(
        archive::get_archive_entry_names(path),
        ["a.txt", "b.txt", "c.txt", "d.txt", "e.txt"]
    );
}

/// Precondition: A solid archive is split into several parts.
/// Action: Run `pna sort` with `--unsolid` and `--output` on the first part.
/// Expectation: The output is a single archive whose entries are sorted by name.
#[test]
fn sort_unsolid_split_solid_archive() {
    setup();
    let dir = "sort_unsolid_split_solid";
    super::keep_solid::create_split_solid_archive(dir);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "--unsolid",
        "-f",
        &format!("{dir}/archive.part1.pna"),
        "--output",
        &format!("{dir}/sorted.pna"),
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::get_archive_entry_names(format!("{dir}/sorted.pna")),
        [format!("{dir}/in/a.txt"), format!("{dir}/in/b.txt"),]
    );
}

/// Precondition: A non-solid archive contains entries in non-alphabetical order.
/// Action: Run `pna sort` with `--unsolid`.
/// Expectation: Entries are sorted by name.
#[test]
fn sort_non_solid_archive_with_unsolid() {
    setup();
    let path = "sort_non_solid_with_unsolid.pna";
    archive::create_archive_with_permissions(
        path,
        &[
            FileEntryDef {
                path: "b.txt",
                content: b"b",
                permission: 0o644,
            },
            FileEntryDef {
                path: "a.txt",
                content: b"a",
                permission: 0o644,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from(["pna", "--quiet", "sort", "--unsolid", "-f", path])
        .unwrap()
        .execute()
        .unwrap();

    assert_eq!(layout(path), [false, false]);
    assert_eq!(archive::get_archive_entry_names(path), ["a.txt", "b.txt"]);
}
