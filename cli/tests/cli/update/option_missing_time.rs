use crate::utils::{archive, setup};
use clap::Parser;
use pna::ReadOptions;
use portable_network_archive::cli;
use std::{fs, io::prelude::*};

/// Precondition: Archive contains an entry without mtime (mTIM chunk omitted).
/// Action: Run `pna experimental update` without `--missing-time`.
/// Expectation: Default `include` policy treats mtime-missing entries as stale,
/// so under append-only update semantics a fresh copy with the modified content
/// is appended; the latest copy of the entry holds `"new content"`.
#[test]
fn update_default_mtime_missing_still_updates() {
    setup();
    let _ = fs::remove_dir_all("update_missing_time_default");
    fs::create_dir_all("update_missing_time_default").unwrap();
    let source = "update_missing_time_default/file.txt";
    let archive_path = "update_missing_time_default/archive.pna";

    // Create an initial archive whose entry has no mTIM chunk.
    fs::write(source, "old content").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        archive_path,
        "--overwrite",
        source,
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Sanity check: entry is mTIM-less.
    archive::for_each_entry(archive_path, |entry| {
        assert!(
            entry.metadata().modified().is_none(),
            "precondition: entry {} must be mTIM-less",
            entry.header().path()
        );
    })
    .unwrap();

    // Modify the source file.
    fs::write(source, "new content").unwrap();

    // Default behavior: no `--missing-time` flag.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        archive_path,
        source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Append-only: collect every copy of the entry and verify the latest one
    // reflects the modified content.
    let mut contents: Vec<Vec<u8>> = Vec::new();
    archive::for_each_entry(archive_path, |entry| {
        if entry.header().path().as_str() == source {
            let mut buf = Vec::new();
            entry
                .reader(ReadOptions::with_password::<&[u8]>(None))
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
            contents.push(buf);
        }
    })
    .unwrap();
    assert!(!contents.is_empty(), "archive should contain {source}");
    assert_eq!(
        contents.last().unwrap().as_slice(),
        b"new content",
        "default policy should append a fresh copy with the modified content"
    );
}

/// Precondition: Archive contains an entry without mtime.
/// Action: Run `pna experimental update --missing-time=exclude` on its own
/// (no time-filter flag). The staleness judgment fires for every entry
/// regardless of time-filter flags, so the missing-time policy takes effect.
/// Expectation: The entry is kept (pass-through) because `exclude` treats mtime-missing
/// archive entries as NOT stale.
#[test]
fn update_missing_time_exclude_keeps_entry() {
    setup();
    let _ = fs::remove_dir_all("update_missing_time_exclude");
    fs::create_dir_all("update_missing_time_exclude").unwrap();
    let source = "update_missing_time_exclude/file.txt";
    let archive_path = "update_missing_time_exclude/archive.pna";

    // Create an initial archive whose entry has no mTIM chunk.
    fs::write(source, "old content").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        archive_path,
        "--overwrite",
        source,
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(archive_path, |entry| {
        assert!(
            entry.metadata().modified().is_none(),
            "precondition: entry {} must be mTIM-less",
            entry.header().path()
        );
    })
    .unwrap();

    // Modify the source file.
    fs::write(source, "new content").unwrap();

    // Run update with --missing-time=exclude alone (no time filter);
    // the staleness judgment consults the policy for every entry.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--unstable",
        "--missing-time=exclude",
        "-f",
        archive_path,
        source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Exclude policy keeps the mtime-less entry untouched: content is OLD.
    let entry = archive::extract_single_entry(archive_path, source)
        .unwrap()
        .expect("entry should exist");
    let mut buf = Vec::new();
    entry
        .reader(ReadOptions::with_password::<&[u8]>(None))
        .unwrap()
        .read_to_end(&mut buf)
        .unwrap();
    assert_eq!(
        buf.as_slice(),
        b"old content",
        "exclude policy should keep mtime-missing entries"
    );
}
