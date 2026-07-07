use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, io::prelude::*, time};

const DURATION_24_HOURS: time::Duration = time::Duration::from_secs(24 * 60 * 60);

/// Precondition: A solid mode archive exists and a source file is modified
/// with a newer mtime.
/// Action: Run `pna experimental update` with `--unsolid`.
/// Expectation: All entries are converted to normal entries, the entry path
/// set is unchanged, and extraction yields the updated content.
#[test]
fn update_with_unsolid() {
    setup();
    TestResources::extract_in("raw/", "update_with_unsolid/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_with_unsolid/archive.pna",
        "--overwrite",
        "--solid",
        "--no-keep-dir",
        "update_with_unsolid/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_with_unsolid/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_unsolid/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated content for unsolid test").unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--unsolid",
        "-f",
        "update_with_unsolid/archive.pna",
        "update_with_unsolid/in/",
        "--no-keep-dir",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut archive = pna::Archive::open("update_with_unsolid/archive.pna").unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry, pna::ReadEntry::Normal(_))),
        "all entries should be converted to normal entries"
    );

    let mut seen = HashSet::new();
    archive::for_each_entry("update_with_unsolid/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert_eq!(
        seen, initial_entries,
        "entry path set should be unchanged after update"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "update_with_unsolid/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_with_unsolid/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert_eq!(
        fs::read("update_with_unsolid/out/raw/text.txt").unwrap(),
        b"updated content for unsolid test",
        "extraction should yield the updated content"
    );
}
