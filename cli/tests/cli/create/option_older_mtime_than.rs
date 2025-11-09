use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    collections::HashSet,
    fs, thread,
    time::{Duration, SystemTime},
};

/// Precondition: Create three files (older, reference, newer) with strictly increasing mtime.
/// Action: Run `pna create` with `--older-mtime-than reference.txt`, specifying all three files.
/// Expectation: Files with mtime <= reference.txt are included (older + reference); newer is excluded.
#[test]
fn create_with_older_mtime_than() {
    setup();
    let reference_file = "create_older_mtime_than/reference.txt";
    let older_file = "create_older_mtime_than/older.txt";
    let newer_file = "create_older_mtime_than/newer.txt";

    fs::create_dir_all("create_older_mtime_than").unwrap();
    fs::write(older_file, "older file content").unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(reference_file, "reference file content").unwrap();
    let reference_mtime = fs::metadata(reference_file).unwrap().modified().unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(newer_file, "newer file content").unwrap();

    if !ensure_mtime_order(older_file, reference_mtime, newer_file) {
        eprintln!(
            "Skipping test: unable to establish strict modification time ordering on this filesystem"
        );
        return;
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_older_mtime_than/test.pna",
        "--overwrite",
        "--unstable",
        "--older-mtime-than",
        reference_file,
        older_file,
        reference_file,
        newer_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("create_older_mtime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert!(
        seen.contains(older_file),
        "older file should be included: {older_file}"
    );
    assert!(
        seen.contains(reference_file),
        "reference file should be included: {reference_file}"
    );
    assert!(
        !seen.contains(newer_file),
        "newer file should NOT be included: {newer_file}"
    );
    assert_eq!(
        seen.len(),
        2,
        "Expected exactly 2 entries, but found {}: {seen:?}",
        seen.len()
    );
}

fn ensure_mtime_order(older: &str, baseline: SystemTime, newer: &str) -> bool {
    if !confirm_mtime_not_newer_than(older, baseline) {
        return false;
    }
    wait_until_mtime_newer_than(newer, baseline)
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

fn confirm_mtime_not_newer_than(path: &str, baseline: SystemTime) -> bool {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .map(|mtime| mtime <= baseline)
        .unwrap_or(false)
}
