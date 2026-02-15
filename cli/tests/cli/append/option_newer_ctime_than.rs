use crate::utils::{archive, setup, time::ensure_ctime_order};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time::Duration};

/// Precondition: Create an archive with an older file, then prepare reference and newer files.
/// Action: Run `pna append` with `--newer-ctime-than reference.txt`, specifying older, reference, and newer files.
/// Expectation: Files with ctime > reference are appended (newer only); older and reference are not re-added.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn append_with_newer_ctime_than() {
    setup();
    let reference_file = "append_newer_ctime_than/reference.txt";
    let older_file = "append_newer_ctime_than/older.txt";
    let newer_file = "append_newer_ctime_than/newer.txt";

    // Create directory
    fs::create_dir_all("append_newer_ctime_than").unwrap();

    // Create the older file
    fs::write(older_file, "older file content").unwrap();

    // Check if creation time is available on this filesystem
    if fs::metadata(older_file).unwrap().created().is_err() {
        eprintln!("Skipping test: creation time (birth time) is not supported on this filesystem");
        return;
    }

    // Create an archive with the older file
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "append_newer_ctime_than/test.pna",
        "--overwrite",
        older_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Wait to ensure distinct ctime
    thread::sleep(Duration::from_millis(10));

    // Create the reference file
    fs::write(reference_file, "reference time marker").unwrap();

    // Wait to ensure the next file has a newer ctime
    thread::sleep(Duration::from_millis(10));

    // Create the newer file
    fs::write(newer_file, "newer file content").unwrap();
    let reference_ctime = fs::metadata(reference_file).unwrap().created().unwrap();
    if !ensure_ctime_order(older_file, newer_file, reference_ctime) {
        eprintln!("Skipping test: unable to produce strict ctime ordering on this filesystem");
        return;
    }

    // Append to the archive with the `--newer-ctime-than` option
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "a",
        "append_newer_ctime_than/test.pna",
        "--no-keep-dir",
        "--unstable",
        "--newer-ctime-than",
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
    archive::for_each_entry("append_newer_ctime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // older_file should be included (from original archive creation)
    assert!(
        seen.contains(older_file),
        "older file should be in archive from initial creation: {older_file}"
    );

    // reference_file should NOT be included (ctime == reference)
    assert!(
        !seen.contains(reference_file),
        "reference file should NOT be appended: {reference_file}"
    );

    // newer_file should be included (appended because ctime > reference)
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
