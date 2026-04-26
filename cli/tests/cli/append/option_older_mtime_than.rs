use crate::utils::{
    archive, setup,
    time::{confirm_time_older_than, wait_until_time_newer_than},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time::Duration};

/// Precondition: An archive exists with an older file, and the source tree contains a reference file and a newer file.
/// Action: Run `pna append` with `--older-mtime-than` pointing to the reference file.
/// Expectation: Only files whose modification time is older than the reference file are appended to the archive.
#[test]
fn append_with_older_mtime_than() {
    setup();
    let base_dir = "append_older_mtime_than";
    let archive_path = format!("{base_dir}/test.pna");
    let older_file = format!("{base_dir}/older.txt");
    let reference_file = format!("{base_dir}/reference.txt");
    let newer_file = format!("{base_dir}/newer.txt");

    fs::create_dir_all(base_dir).unwrap();
    fs::write(&older_file, "older mtime content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &archive_path,
        "--overwrite",
        &older_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(&reference_file, "reference mtime content").unwrap();
    let reference_mtime = fs::metadata(&reference_file).unwrap().modified().unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(&newer_file, "newer mtime content").unwrap();

    if !confirm_time_older_than(&older_file, reference_mtime, |m| m.modified().ok())
        || !wait_until_time_newer_than(&newer_file, reference_mtime, |m| m.modified().ok())
    {
        eprintln!(
            "Skipping test: unable to establish required modification time ordering on this filesystem"
        );
        return;
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--no-keep-dir",
        "--unstable",
        "--older-mtime-than",
        &reference_file,
        "-f",
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
