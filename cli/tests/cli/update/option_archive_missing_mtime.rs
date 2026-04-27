use crate::utils::{archive, setup};
use clap::Parser;
use pna::ReadOptions;
use portable_network_archive::cli;
use std::{fs, io::prelude::*};

/// Precondition: Archive contains an entry without mtime (mTIM chunk omitted).
/// Action: Run `pna experimental update` without `--archive-missing-mtime`.
/// Expectation: The entry is replaced with the newer source content
/// (default `Include` policy treats mtime-missing entries as stale, reproducing the
/// pre-change hardcoded behavior).
#[test]
fn update_default_mtime_missing_still_updates() {
    setup();
    let _ = fs::remove_dir_all("update_arc_missing_mtime_default");
    fs::create_dir_all("update_arc_missing_mtime_default").unwrap();
    let source = "update_arc_missing_mtime_default/file.txt";
    let archive_path = "update_arc_missing_mtime_default/archive.pna";

    // Create an initial archive whose entry has no mTIM chunk.
    fs::write(source, "old content").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
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

    // Default behavior: no `--archive-missing-mtime` flag.
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

    // Read the entry content from the archive and confirm it was updated.
    let entry = archive::extract_single_entry(archive_path, source)
        .unwrap()
        .expect("entry should exist");
    let mut buf = Vec::new();
    entry
        .reader(&mut ReadOptions::with_password::<&[u8]>(None))
        .unwrap()
        .read_to_end(&mut buf)
        .unwrap();
    assert_eq!(
        buf.as_slice(),
        b"new content",
        "default policy should update mtime-missing entries"
    );
}

/// Precondition: Archive contains an entry without mtime.
/// Action: Run `pna experimental update --archive-missing-mtime=exclude` on its own
/// (no time-filter flag). Update's Path B staleness judgment fires for every entry
/// regardless of time-filter flags, so the archive-missing policy takes effect.
/// Expectation: The entry is kept (pass-through) because `exclude` treats mtime-missing
/// archive entries as NOT stale.
#[test]
fn update_archive_missing_mtime_exclude_keeps_entry() {
    setup();
    let _ = fs::remove_dir_all("update_arc_missing_mtime_exclude");
    fs::create_dir_all("update_arc_missing_mtime_exclude").unwrap();
    let source = "update_arc_missing_mtime_exclude/file.txt";
    let archive_path = "update_arc_missing_mtime_exclude/archive.pna";

    // Create an initial archive whose entry has no mTIM chunk.
    fs::write(source, "old content").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
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

    // Run update with --archive-missing-mtime=exclude alone (no time filter);
    // Path B staleness judgment consults the policy for every entry.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--unstable",
        "--archive-missing-mtime=exclude",
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
        .reader(&mut ReadOptions::with_password::<&[u8]>(None))
        .unwrap()
        .read_to_end(&mut buf)
        .unwrap();
    assert_eq!(
        buf.as_slice(),
        b"old content",
        "exclude policy should keep mtime-missing entries"
    );
}
