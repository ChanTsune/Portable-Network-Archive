use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, io::prelude::*, path::Path, time};

const DURATION_24_HOURS: time::Duration = time::Duration::from_secs(24 * 60 * 60);

/// Precondition: The source tree is archived and split into multiple parts,
/// then a source file is modified with a newer mtime.
/// Action: Run `pna experimental update` on the first part of the multipart archive.
/// Expectation: All parts are read as one archive, the result is written as a
/// single unsplit archive, the entry set is unchanged, and extraction yields
/// the updated content. The original part files remain on disk.
#[test]
fn update_multipart_archive() {
    setup();
    TestResources::extract_in("raw/", "update_multipart/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_multipart/archive.pna",
        "--overwrite",
        "update_multipart/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "-f",
        "update_multipart/archive.pna",
        "--overwrite",
        "--max-size",
        "1kb",
        "--out-dir",
        "update_multipart/split/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(
        Path::new("update_multipart/split/archive.part2.pna").exists(),
        "precondition: the archive should be split into multiple parts"
    );

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_multipart/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_multipart/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated content for multipart test")
        .unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_multipart/split/archive.part1.pna",
        "update_multipart/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Rewriting commands persist to the part-less path, merging all parts.
    assert!(
        Path::new("update_multipart/split/archive.pna").exists(),
        "update on a multipart archive should write a single unsplit archive"
    );
    assert!(
        Path::new("update_multipart/split/archive.part1.pna").exists()
            && Path::new("update_multipart/split/archive.part2.pna").exists(),
        "original part files should remain on disk"
    );

    let mut seen = HashSet::new();
    archive::for_each_entry("update_multipart/split/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert_eq!(
        seen, initial_entries,
        "entry set should be unchanged after update"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "update_multipart/split/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_multipart/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert_eq!(
        fs::read("update_multipart/out/raw/text.txt").unwrap(),
        b"updated content for multipart test",
        "extraction should yield the updated content"
    );
}

/// Precondition: The source tree is archived and split into multiple parts,
/// then a source file is deleted from disk.
/// Action: Run `pna experimental update --sync` on the first part of the
/// multipart archive.
/// Expectation: Only the entry whose file is missing on disk is pruned from
/// the merged archive; entries from all parts are otherwise preserved.
#[test]
fn update_multipart_archive_with_sync() {
    setup();
    TestResources::extract_in("raw/", "update_multipart_sync/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_multipart_sync/archive.pna",
        "--overwrite",
        "update_multipart_sync/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "-f",
        "update_multipart_sync/archive.pna",
        "--overwrite",
        "--max-size",
        "1kb",
        "--out-dir",
        "update_multipart_sync/split/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(
        Path::new("update_multipart_sync/split/archive.part2.pna").exists(),
        "precondition: the archive should be split into multiple parts"
    );

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_multipart_sync/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    fs::remove_file("update_multipart_sync/in/raw/empty.txt").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--sync",
        "-f",
        "update_multipart_sync/split/archive.part1.pna",
        "update_multipart_sync/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_multipart_sync/split/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    let mut expected = initial_entries;
    assert!(
        expected.remove("update_multipart_sync/in/raw/empty.txt"),
        "precondition: initial archive should contain the deleted file"
    );
    assert_eq!(
        seen, expected,
        "only the entry missing on disk should be pruned from the merged archive"
    );
}
