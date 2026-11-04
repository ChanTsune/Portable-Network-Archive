use crate::utils::{
    archive, setup,
    time::{DURATION_24_HOURS, set_mtime},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, time::SystemTime};

/// Precondition: An archive exists with an older file, and the source tree contains a reference file and a newer file.
/// Action: Run `pna append` with `--newer-mtime-than` pointing to the reference file.
/// Expectation: Only files whose modification time is newer than the reference file are appended to the archive.
#[test]
fn append_with_newer_mtime_than() {
    setup();
    let reference_file = "append_newer_mtime_than/reference.txt";
    let older_file = "append_newer_mtime_than/older.txt";
    let newer_file = "append_newer_mtime_than/newer.txt";

    fs::create_dir_all("append_newer_mtime_than").unwrap();
    fs::write(older_file, "older file content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "append_newer_mtime_than/test.pna",
        "--overwrite",
        older_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let now = SystemTime::now();
    fs::write(reference_file, "reference time marker").unwrap();
    fs::write(newer_file, "newer file content").unwrap();
    set_mtime(older_file, now - 2 * DURATION_24_HOURS);
    set_mtime(reference_file, now - DURATION_24_HOURS);
    set_mtime(newer_file, now);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "a",
        "-f",
        "append_newer_mtime_than/test.pna",
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

    let mut seen = HashSet::new();
    archive::for_each_entry("append_newer_mtime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(
        seen,
        HashSet::from([older_file.to_string(), newer_file.to_string()])
    );
}
