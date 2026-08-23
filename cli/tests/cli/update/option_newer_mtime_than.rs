use crate::utils::{
    archive, setup,
    time::{DURATION_24_HOURS, set_mtime},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, time::SystemTime};

/// Precondition: An archive exists with files to update, and the source tree contains a reference file and files with varying modification times.
/// Action: Run `pna experimental update` with `--newer-mtime-than` pointing to the reference file.
/// Expectation: Only files whose modification time is newer than the reference file are updated or added to the archive.
#[test]
fn update_with_newer_mtime_than() {
    setup();
    let reference_file = "update_newer_mtime_than/reference.txt";
    let file_to_update = "update_newer_mtime_than/file_to_update.txt";
    let file_to_add = "update_newer_mtime_than/file_to_add.txt";

    fs::create_dir_all("update_newer_mtime_than").unwrap();
    fs::write(file_to_update, "initial content").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_newer_mtime_than/test.pna",
        "--overwrite",
        file_to_update,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let now = SystemTime::now();
    fs::write(reference_file, "time reference").unwrap();
    fs::write(file_to_update, "updated content").unwrap();
    fs::write(file_to_add, "new file content").unwrap();
    set_mtime(reference_file, now - DURATION_24_HOURS);
    set_mtime(file_to_update, now);
    set_mtime(file_to_add, now);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--overwrite",
        "--file",
        "update_newer_mtime_than/test.pna",
        file_to_update,
        file_to_add,
        reference_file,
        "--unstable",
        "--newer-mtime-than",
        reference_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_newer_mtime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(
        seen,
        HashSet::from([file_to_update.to_string(), file_to_add.to_string()])
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--file",
        "update_newer_mtime_than/test.pna",
        "--out-dir",
        "update_newer_mtime_than/out",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content = fs::read_to_string(
        "update_newer_mtime_than/out/update_newer_mtime_than/file_to_update.txt",
    )
    .unwrap();
    assert_eq!(
        updated_content, "updated content",
        "The updated file did not contain the correct content"
    );
}
