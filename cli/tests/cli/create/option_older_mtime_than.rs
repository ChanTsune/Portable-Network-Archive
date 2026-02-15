use crate::utils::{
    archive, setup,
    time::{confirm_time_older_than, wait_until_time_newer_than},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time::Duration};

/// Precondition: Create three files (older, reference, newer) with strictly increasing mtime.
/// Action: Run `pna create` with `--older-mtime-than reference.txt`, specifying all three files.
/// Expectation: Files with mtime < reference.txt are included (older only); reference and newer are excluded.
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

    if !confirm_time_older_than(older_file, reference_mtime, |m| m.modified().ok())
        || !wait_until_time_newer_than(newer_file, reference_mtime, |m| m.modified().ok())
    {
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
        "--no-keep-dir",
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
        !seen.contains(reference_file),
        "reference file should NOT be included: {reference_file}"
    );
    assert!(
        !seen.contains(newer_file),
        "newer file should NOT be included: {newer_file}"
    );
    assert_eq!(
        seen.len(),
        1,
        "Expected exactly 1 entry, but found {}: {seen:?}",
        seen.len()
    );
}
