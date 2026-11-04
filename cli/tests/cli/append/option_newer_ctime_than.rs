use crate::utils::{
    archive, setup,
    time::{birth_time, birth_time_recorded, create_file_born_after},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: An archive exists with an older file, and the source tree contains a reference file and a newer file.
/// Action: Run `pna append` with `--newer-ctime-than` pointing to the reference file.
/// Expectation: Only files whose creation time is newer than the reference file are appended to the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn append_with_newer_ctime_than() {
    setup();
    let reference_file = "append_newer_ctime_than/reference.txt";
    let older_file = "append_newer_ctime_than/older.txt";
    let newer_file = "append_newer_ctime_than/newer.txt";

    fs::create_dir_all("append_newer_ctime_than").unwrap();
    fs::write(older_file, "older file content").unwrap();

    skip_unless!("birthtime", birth_time_recorded(older_file));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "append_newer_ctime_than/test.pna",
        "--overwrite",
        older_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let reference_ctime = create_file_born_after(
        reference_file,
        "reference time marker",
        birth_time(older_file),
    );
    create_file_born_after(newer_file, "newer file content", reference_ctime);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "a",
        "-f",
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

    let mut seen = HashSet::new();
    archive::for_each_entry("append_newer_ctime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(
        seen,
        HashSet::from([older_file.to_string(), newer_file.to_string()])
    );
}
