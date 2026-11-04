use crate::utils::{EmbedExt, TestResources, archive, setup, time::DURATION_24_HOURS};
use clap::Parser;
use pna::ReadOptions;
use portable_network_archive::cli;
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::prelude::*,
};

/// Precondition: An archive created with `--no-keep-timestamp` (entries have no mtime).
/// Action: Modify a file and run `pna experimental update` with `--no-keep-timestamp`.
/// Expectation: Under append-only update semantics, the original entry remains
///   and a fresh copy with the modified content is appended; the latest copy
///   reflects the modification.
#[test]
fn update_no_timestamp_archive_always_updates() {
    setup();
    let _ = fs::remove_dir_all("update_no_ts_always");
    TestResources::extract_in("raw/", "update_no_ts_always/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_no_ts_always/archive.pna",
        "--overwrite",
        "update_no_ts_always/in/",
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut initial_entries = HashSet::new();
    let mut text_txt_path = String::new();
    archive::for_each_entry("update_no_ts_always/archive.pna", |entry| {
        assert!(
            entry.metadata().modified().is_none(),
            "entry {} should have no mtime after --no-keep-timestamp",
            entry.header().path()
        );
        let path = entry.header().path().to_string();
        if path.ends_with("raw/text.txt") {
            text_txt_path = path.clone();
        }
        initial_entries.insert(path);
    })
    .unwrap();
    assert!(
        !initial_entries.is_empty(),
        "archive should contain entries"
    );
    assert!(!text_txt_path.is_empty(), "archive should contain text.txt");

    let updated_content = b"content written before update";
    fs::write("update_no_ts_always/in/raw/text.txt", updated_content).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_no_ts_always/archive.pna",
        "update_no_ts_always/in/",
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut post_entries = HashSet::new();
    let mut text_txt_contents: Vec<Vec<u8>> = Vec::new();
    archive::for_each_entry("update_no_ts_always/archive.pna", |entry| {
        post_entries.insert(entry.header().path().to_string());
        if entry.header().path().as_str() == text_txt_path {
            let mut buf = Vec::new();
            entry
                .reader(ReadOptions::with_password::<&[u8]>(None))
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
            text_txt_contents.push(buf);
        }
    })
    .unwrap();
    assert!(!text_txt_contents.is_empty(), "text.txt entry should exist");
    // Append-only semantics: the latest text.txt copy reflects the modification.
    assert_eq!(
        text_txt_contents.last().unwrap().as_slice(),
        updated_content,
        "the most recent text.txt copy should reflect the modification"
    );
    assert_eq!(
        initial_entries, post_entries,
        "update should preserve every original path-level entry"
    );
}

/// Precondition: An archive created with `--no-keep-timestamp` (entries have no mtime).
/// Action: Delete a source file and run `pna experimental update --sync`.
/// Expectation: Deleted file is removed from archive; remaining entries preserved.
#[test]
fn update_no_timestamp_archive_with_sync() {
    setup();
    let _ = fs::remove_dir_all("update_no_ts_sync");
    TestResources::extract_in("raw/", "update_no_ts_sync/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_no_ts_sync/archive.pna",
        "--overwrite",
        "update_no_ts_sync/in/",
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_no_ts_sync/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert!(
        initial_entries.iter().any(|p| p.ends_with("raw/empty.txt")),
        "initial archive should contain empty.txt"
    );
    let initial_count = initial_entries.len();

    fs::remove_file("update_no_ts_sync/in/raw/empty.txt").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_no_ts_sync/archive.pna",
        "update_no_ts_sync/in/",
        "--no-keep-timestamp",
        "--sync",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut post_entries = HashSet::new();
    archive::for_each_entry("update_no_ts_sync/archive.pna", |entry| {
        post_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert!(
        !post_entries.iter().any(|p| p.ends_with("raw/empty.txt")),
        "empty.txt should be removed with --sync, but found entries: {post_entries:?}"
    );
    assert_eq!(
        post_entries.len(),
        initial_count - 1,
        "entry count should decrease by 1 with --sync"
    );
}

/// Precondition: An archive created with `--no-keep-timestamp` (entries have no mtime).
/// Action: Run `pna experimental update` with `--keep-timestamp`.
/// Expectation: Under append-only update semantics, the original mtime-less
///   entries are preserved while newly appended copies acquire mtime from the
///   filesystem.
#[test]
fn update_no_timestamp_archive_gains_mtime_with_keep_timestamp() {
    setup();
    let _ = fs::remove_dir_all("update_no_ts_gains_mtime");
    TestResources::extract_in("raw/", "update_no_ts_gains_mtime/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_no_ts_gains_mtime/archive.pna",
        "--overwrite",
        "update_no_ts_gains_mtime/in/",
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("update_no_ts_gains_mtime/archive.pna", |entry| {
        assert!(
            entry.metadata().modified().is_none(),
            "entry {} should have no mtime initially",
            entry.header().path()
        );
    })
    .unwrap();

    // entries lack mtime -> always updated -> gain fs mtime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_no_ts_gains_mtime/archive.pna",
        "update_no_ts_gains_mtime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut has_mtime_entry = false;
    let mut has_no_mtime_entry = false;
    archive::for_each_entry("update_no_ts_gains_mtime/archive.pna", |entry| {
        if entry.metadata().modified().is_some() {
            has_mtime_entry = true;
        } else {
            has_no_mtime_entry = true;
        }
    })
    .unwrap();
    assert!(
        has_mtime_entry,
        "newly appended entries should gain mtime with --keep-timestamp"
    );
    assert!(
        has_no_mtime_entry,
        "original mtime-less entries are preserved under append-only update"
    );
}

/// Precondition: An archive created with `--no-keep-timestamp` (entries have no mtime).
/// Action: Run `pna experimental update` with `--no-keep-timestamp`.
/// Expectation: All re-archived entries have no mtime.
#[test]
fn update_no_timestamp_archive_stays_without_mtime() {
    setup();
    let _ = fs::remove_dir_all("update_no_ts_stays");
    TestResources::extract_in("raw/", "update_no_ts_stays/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_no_ts_stays/archive.pna",
        "--overwrite",
        "update_no_ts_stays/in/",
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_no_ts_stays/archive.pna",
        "update_no_ts_stays/in/",
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut has_entries = false;
    archive::for_each_entry("update_no_ts_stays/archive.pna", |entry| {
        has_entries = true;
        assert!(
            entry.metadata().modified().is_none(),
            "entry {} should have no mtime after update with --no-keep-timestamp",
            entry.header().path()
        );
    })
    .unwrap();
    assert!(has_entries, "archive should contain entries");
}

/// Precondition: An archive created with `--keep-timestamp` (entries have mtime).
/// Action: Modify a file, run `pna experimental update` with `--no-keep-timestamp`.
/// Expectation: Under append-only update semantics, the original mtimed copy
///   of the modified file is preserved and a fresh mtime-less copy is appended;
///   unmodified entries pass through with original mtime unchanged.
#[test]
fn update_timestamped_archive_loses_mtime_with_no_keep_timestamp() {
    setup();
    let _ = fs::remove_dir_all("update_ts_loses_mtime");
    TestResources::extract_in("raw/", "update_ts_loses_mtime/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_ts_loses_mtime/archive.pna",
        "--overwrite",
        "update_ts_loses_mtime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut initial_mtimes = BTreeMap::new();
    let mut text_txt_path = String::new();
    archive::for_each_entry("update_ts_loses_mtime/archive.pna", |entry| {
        let path = entry.header().path().to_string();
        if path.ends_with("raw/text.txt") {
            text_txt_path = path.clone();
        }
        initial_mtimes.insert(path, entry.metadata().modified());
    })
    .unwrap();
    assert!(
        initial_mtimes.values().any(|m| m.is_some()),
        "initial archive should have entries with mtime"
    );
    assert!(!text_txt_path.is_empty(), "archive should contain text.txt");

    // Set mtime far in future to guarantee detection as newer
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_ts_loses_mtime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated content").unwrap();
    file.set_modified(std::time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();
    drop(file);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_ts_loses_mtime/archive.pna",
        "update_ts_loses_mtime/in/",
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Updated entry (text.txt) is appended without mtime; the original mtimed
    // copy is preserved alongside it. Non-updated entries pass through unchanged.
    let mut text_txt_mtimes: Vec<Option<pna::Duration>> = Vec::new();
    archive::for_each_entry("update_ts_loses_mtime/archive.pna", |entry| {
        let path = entry.header().path().to_string();
        if path == text_txt_path {
            text_txt_mtimes.push(entry.metadata().modified());
        } else {
            assert_eq!(
                entry.metadata().modified(),
                *initial_mtimes.get(&path).unwrap(),
                "non-updated entry {path} should pass through with original mtime"
            );
        }
    })
    .unwrap();
    assert!(
        text_txt_mtimes.len() >= 2,
        "text.txt should have an original copy plus an appended copy under append-only update"
    );
    assert_eq!(
        text_txt_mtimes.last().copied(),
        Some(None),
        "the latest text.txt copy must have no mtime under --no-keep-timestamp"
    );
}
