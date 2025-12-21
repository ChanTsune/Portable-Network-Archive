use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{
    collections::HashSet,
    fs, thread,
    time::{Duration, SystemTime},
};

/// Precondition: Create an archive with an older file, then prepare reference and newer files.
/// Action: Run `pna append` with `--newer-mtime-than reference.txt`, specifying older, reference, and newer files.
/// Expectation: Files with mtime > reference are appended (newer only); older and reference are not re-added.
#[test]
fn append_with_newer_mtime_than() {
    setup();
    let reference_file = "append_newer_mtime_than/reference.txt";
    let older_file = "append_newer_mtime_than/older.txt";
    let newer_file = "append_newer_mtime_than/newer.txt";

    // Create directory
    fs::create_dir_all("append_newer_mtime_than").unwrap();

    // Create the older file
    fs::write(older_file, "older file content").unwrap();

    // Create an archive with the older file
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "append_newer_mtime_than/test.pna",
        "--overwrite",
        older_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Wait to ensure distinct mtime
    thread::sleep(Duration::from_millis(10));

    // Create the reference file
    fs::write(reference_file, "reference time marker").unwrap();

    // Wait to ensure the next file has a newer mtime
    thread::sleep(Duration::from_millis(10));

    // Create the newer file
    fs::write(newer_file, "newer file content").unwrap();
    let reference_mtime = fs::metadata(reference_file).unwrap().modified().unwrap();
    if !ensure_mtime_order(older_file, newer_file, reference_mtime) {
        eprintln!("Skipping test: unable to produce strict mtime ordering on this filesystem");
        return;
    }

    // Append to the archive with the `--newer-mtime-than` option
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "a",
        "append_newer_mtime_than/test.pna",
        "--unstable",
        "--newer-mtime-than",
        reference_file,
        older_file,
        reference_file,
        newer_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents
    let mut seen = HashSet::new();
    archive::for_each_entry("append_newer_mtime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // older_file should be included (from original archive creation)
    assert!(
        seen.contains(older_file),
        "older file should be in archive from initial creation: {older_file}"
    );

    // reference_file should NOT be included (mtime == reference)
    assert!(
        !seen.contains(reference_file),
        "reference file should NOT be appended: {reference_file}"
    );

    // newer_file should be included (appended because mtime > reference)
    assert!(
        seen.contains(newer_file),
        "newer file should be appended: {newer_file}"
    );

    // Verify that exactly two entries exist
    assert_eq!(
        seen.len(),
        2,
        "Expected exactly 2 entries, but found {}: {seen:?}",
        seen.len()
    );
}

fn ensure_mtime_order(older: &str, newer: &str, reference: SystemTime) -> bool {
    if !confirm_mtime_older_than(older, reference) {
        return false;
    }
    wait_until_mtime_newer_than(newer, reference)
}

fn wait_until_mtime_newer_than(path: &str, baseline: SystemTime) -> bool {
    const MAX_ATTEMPTS: usize = 500;
    const SLEEP_MS: u64 = 10;
    for _ in 0..MAX_ATTEMPTS {
        if fs::metadata(path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .map(|mtime| mtime > baseline)
            .unwrap_or(false)
        {
            return true;
        }
        thread::sleep(Duration::from_millis(SLEEP_MS));
    }
    false
}

fn confirm_mtime_older_than(path: &str, baseline: SystemTime) -> bool {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .map(|mtime| mtime < baseline)
        .unwrap_or(false)
}
