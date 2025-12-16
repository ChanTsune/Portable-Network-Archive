use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{
    collections::HashSet,
    fs, thread,
    time::{Duration, SystemTime},
};

/// Precondition: Archive already contains `older.txt`. Prepare `reference.txt` and `newer.txt` with
/// strictly ordered creation times (older < reference < newer).
/// Action: Run `pna append` with `--older-ctime-than reference.txt`, appending both candidate files.
/// Expectation: Only files whose ctime <= reference (i.e., reference itself) are appended; `newer` is skipped.
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

    if !confirm_ctime_not_newer_than(&older_file, reference_ctime)
        || !wait_until_ctime_newer_than(&newer_file, reference_ctime)
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
        seen.contains(&reference_file),
        "reference file should be appended: {reference_file}"
    );
    assert!(
        !seen.contains(&newer_file),
        "newer file should NOT be appended: {newer_file}"
    );
    assert_eq!(
        seen.len(),
        2,
        "Expected exactly 2 entries (older + reference) but found {}: {seen:?}",
        seen.len()
    );
}

fn wait_until_ctime_newer_than(path: &str, baseline: SystemTime) -> bool {
    const MAX_ATTEMPTS: usize = 500;
    const SLEEP_MS: u64 = 10;
    for _ in 0..MAX_ATTEMPTS {
        if fs::metadata(path)
            .ok()
            .and_then(|meta| meta.created().ok())
            .map(|ctime| ctime > baseline)
            .unwrap_or(false)
        {
            return true;
        }
        thread::sleep(Duration::from_millis(SLEEP_MS));
    }
    false
}

fn confirm_ctime_not_newer_than(path: &str, baseline: SystemTime) -> bool {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.created().ok())
        .map(|ctime| ctime <= baseline)
        .unwrap_or(false)
}
