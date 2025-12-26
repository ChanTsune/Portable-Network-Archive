use crate::utils::{archive, setup, time::ensure_mtime_order};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time::Duration};

/// Precondition: Create three files with different modification times (reference, older, newer).
/// Action: Run `pna create` with `--newer-mtime-than reference.txt`, specifying all three files.
/// Expectation: Files with mtime > reference.txt are included (newer only); older and reference are excluded.
#[test]
fn create_with_newer_mtime_than() {
    setup();
    let reference_file = "create_newer_mtime_than/reference.txt";
    let older_file = "create_newer_mtime_than/older.txt";
    let newer_file = "create_newer_mtime_than/newer.txt";

    // Create the older file first
    fs::create_dir_all("create_newer_mtime_than").unwrap();
    fs::write(older_file, "older file content").unwrap();

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

    // Create an archive with the `--newer-mtime-than` option
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_newer_mtime_than/test.pna",
        "--overwrite",
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
    archive::for_each_entry("create_newer_mtime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // newer_file should be included (mtime > reference)
    assert!(
        seen.contains(newer_file),
        "newer file should be included: {newer_file}"
    );

    // reference_file should NOT be included (mtime == reference)
    assert!(
        !seen.contains(reference_file),
        "reference file should NOT be included: {reference_file}"
    );

    // older_file should NOT be included (mtime < reference)
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
