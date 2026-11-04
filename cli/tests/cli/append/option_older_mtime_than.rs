use crate::utils::{
    archive, setup,
    time::{DURATION_24_HOURS, set_mtime},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, time::SystemTime};

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

    let now = SystemTime::now();
    fs::write(&reference_file, "reference mtime content").unwrap();
    fs::write(&newer_file, "newer mtime content").unwrap();
    set_mtime(&older_file, now - 2 * DURATION_24_HOURS);
    set_mtime(&reference_file, now - DURATION_24_HOURS);
    set_mtime(&newer_file, now);

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

    assert_eq!(seen, HashSet::from([older_file]));
}
