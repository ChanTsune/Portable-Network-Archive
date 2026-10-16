use crate::utils::{
    archive, setup,
    time::{DURATION_24_HOURS, set_mtime},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, time::SystemTime};

/// Precondition: The source tree contains files with strictly ordered modification times and a reference file.
/// Action: Run `pna create` with `--older-mtime-than` pointing to the reference file.
/// Expectation: Only files whose modification time is older than the reference file are included in the archive.
#[test]
fn create_with_older_mtime_than() {
    setup();
    let reference_file = "create_older_mtime_than/reference.txt";
    let older_file = "create_older_mtime_than/older.txt";
    let newer_file = "create_older_mtime_than/newer.txt";

    let now = SystemTime::now();
    fs::create_dir_all("create_older_mtime_than").unwrap();
    fs::write(older_file, "older file content").unwrap();
    fs::write(reference_file, "reference file content").unwrap();
    fs::write(newer_file, "newer file content").unwrap();
    set_mtime(older_file, now - 2 * DURATION_24_HOURS);
    set_mtime(reference_file, now - DURATION_24_HOURS);
    set_mtime(newer_file, now);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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

    assert_eq!(seen, HashSet::from([older_file.to_string()]));
}
