use crate::utils::{
    archive, setup,
    time::{birth_time, birth_time_recorded, create_file_born_after},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: An archive exists with an older file, and the source tree contains a reference file and a newer file.
/// Action: Run `pna append` with `--older-ctime-than` pointing to the reference file.
/// Expectation: Only files whose creation time is older than the reference file are appended to the archive.
/// Note: This test requires filesystem support for creation time (birth time).
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

    skip_unless!("birthtime", birth_time_recorded(&older_file));

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

    let reference_ctime = create_file_born_after(
        &reference_file,
        "reference content",
        birth_time(&older_file),
    );
    create_file_born_after(&newer_file, "newer content", reference_ctime);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--no-keep-dir",
        "--unstable",
        "--older-ctime-than",
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
