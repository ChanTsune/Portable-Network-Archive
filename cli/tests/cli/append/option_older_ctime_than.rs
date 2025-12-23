use crate::utils::{
    archive, setup,
    time::{confirm_time_older_than, wait_until_time_newer_than},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time::Duration};

/// Precondition: Archive already contains `older.txt`. Prepare `reference.txt` and `newer.txt` with
/// strictly ordered creation times (older < reference < newer).
/// Action: Run `pna append` with `--older-ctime-than reference.txt`, appending both candidate files.
/// Expectation: Only files whose ctime < reference are appended; `reference` and `newer` are skipped.
/// Note: Requires filesystem support for birth time.
#[test]
fn append_with_older_ctime_than() {
    setup();
    let base_dir = "append_older_ctime_than";
    let archive_path = format!("{base_dir}/test.pna");
    let older_file = format!("{base_dir}/older.txt");
    let reference_file = format!("{base_dir}/reference.txt");
    let newer_file = format!("{base_dir}/newer.txt");

    fs::create_dir_all(base_dir).unwrap();
    fs::write(&older_file, "older file content").unwrap();

    if fs::metadata(&older_file).unwrap().created().is_err() {
        eprintln!("Skipping test: creation time (birth time) is not supported on this filesystem");
        return;
    }

    // Create the initial archive containing only the older file.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        &older_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(&reference_file, "reference content").unwrap();
    let reference_ctime = fs::metadata(&reference_file).unwrap().created().unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(&newer_file, "newer content").unwrap();

    if !confirm_time_older_than(&older_file, reference_ctime, |m| m.created().ok())
        || !wait_until_time_newer_than(&newer_file, reference_ctime, |m| m.created().ok())
    {
        eprintln!(
            "Skipping test: unable to establish required creation time ordering on this filesystem"
        );
        return;
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--unstable",
        "--older-ctime-than",
        &reference_file,
        &archive_path,
        &reference_file,
        &newer_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(&archive_path, |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert!(
        seen.contains(&older_file),
        "older file should remain from initial archive: {older_file}"
    );
    assert!(
        !seen.contains(&reference_file),
        "reference file should NOT be appended: {reference_file}"
    );
    assert!(
        !seen.contains(&newer_file),
        "newer file should NOT be appended: {newer_file}"
    );
    assert_eq!(
        seen.len(),
        1,
        "Expected exactly 1 entry (older only) but found {}: {seen:?}",
        seen.len()
    );
}
