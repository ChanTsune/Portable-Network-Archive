use crate::utils::{
    archive, setup,
    time::{confirm_time_older_than, wait_until_time_newer_than},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time::Duration};

/// Precondition: Create three files with different creation times (reference, older, newer).
/// Action: Run `pna create` with `--newer-ctime-than reference.txt`, specifying all three files.
/// Expectation: Files with ctime > reference.txt are included (newer only); older and reference are excluded.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn create_with_newer_ctime_than() {
    setup();
    let reference_file = "create_newer_ctime_than/reference.txt";
    let older_file = "create_newer_ctime_than/older.txt";
    let newer_file = "create_newer_ctime_than/newer.txt";

    // Create the older file first
    fs::create_dir_all("create_newer_ctime_than").unwrap();
    fs::write(older_file, "older file content").unwrap();

    // Check if creation time is available on this filesystem
    if fs::metadata(older_file).unwrap().created().is_err() {
        eprintln!("Skipping test: creation time (birth time) is not supported on this filesystem");
        return;
    }

    // Wait to ensure distinct ctime
    thread::sleep(Duration::from_millis(10));

    // Create the reference file
    fs::write(reference_file, "reference time marker").unwrap();
    let reference_ctime = fs::metadata(reference_file).unwrap().created().unwrap();

    // Wait to ensure the next file has a newer ctime
    thread::sleep(Duration::from_millis(10));

    // Create the newer file
    fs::write(newer_file, "newer file content").unwrap();
    if !wait_until_time_newer_than(newer_file, reference_ctime, |m| m.created().ok())
        || !confirm_time_older_than(older_file, reference_ctime, |m| m.created().ok())
    {
        eprintln!("Skipping test: unable to produce distinct creation times on this filesystem");
        return;
    }

    // Create an archive with the `--newer-ctime-than` option
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_newer_ctime_than/test.pna",
        "--overwrite",
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
    archive::for_each_entry("create_newer_ctime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // newer_file should be included (ctime > reference)
    assert!(
        seen.contains(newer_file),
        "newer file should be included: {newer_file}"
    );

    // reference_file should NOT be included (ctime == reference)
    assert!(
        !seen.contains(reference_file),
        "reference file should NOT be included: {reference_file}"
    );

    // older_file should NOT be included (ctime < reference)
    assert!(
        !seen.contains(older_file),
        "older file should NOT be included: {older_file}"
    );

    // Verify that exactly one entry exists (newer only)
    assert_eq!(
        seen.len(),
        1,
        "Expected exactly 1 entry, but found {}: {seen:?}",
        seen.len()
    );
}
