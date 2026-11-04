use crate::utils::{
    archive, setup,
    time::{birth_time, birth_time_recorded, create_file_born_after},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: The source tree contains files with strictly ordered creation times and a reference file.
/// Action: Run `pna create` with `--newer-ctime-than` pointing to the reference file.
/// Expectation: Only files whose creation time is newer than the reference file are included in the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn create_with_newer_ctime_than() {
    setup();
    let reference_file = "create_newer_ctime_than/reference.txt";
    let older_file = "create_newer_ctime_than/older.txt";
    let newer_file = "create_newer_ctime_than/newer.txt";

    fs::create_dir_all("create_newer_ctime_than").unwrap();
    fs::write(older_file, "older file content").unwrap();

    skip_unless!("birthtime", birth_time_recorded(older_file));

    let reference_ctime = create_file_born_after(
        reference_file,
        "reference time marker",
        birth_time(older_file),
    );
    create_file_born_after(newer_file, "newer file content", reference_ctime);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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

    let mut seen = HashSet::new();
    archive::for_each_entry("create_newer_ctime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(seen, HashSet::from([newer_file.to_string()]));
}
