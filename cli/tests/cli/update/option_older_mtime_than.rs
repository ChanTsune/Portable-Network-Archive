use crate::utils::{
    archive, setup,
    time::{DURATION_24_HOURS, set_mtime},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, time::SystemTime};

/// Precondition: An archive exists with files to update, and the source tree contains a reference file and files with varying modification times.
/// Action: Run `pna experimental update` with `--older-mtime-than` pointing to the reference file.
/// Expectation: Only files whose modification time is older than the reference file are processed in the archive.
#[test]
fn update_with_older_mtime_than() {
    setup();
    let base_dir = "update_older_mtime_than";
    let archive_path = format!("{base_dir}/test.pna");
    let file_to_update = format!("{base_dir}/file_to_update.txt");
    let file_to_skip = format!("{base_dir}/file_to_skip.txt");
    let reference_file = format!("{base_dir}/reference.txt");

    fs::create_dir_all(base_dir).unwrap();
    fs::write(&file_to_update, "initial content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &archive_path,
        "--overwrite",
        &file_to_update,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let now = SystemTime::now();
    fs::write(&file_to_update, "updated content").unwrap();
    fs::write(&reference_file, "reference marker").unwrap();
    fs::write(&file_to_skip, "skip content").unwrap();
    set_mtime(&file_to_update, now - DURATION_24_HOURS);
    set_mtime(&reference_file, now);
    set_mtime(&file_to_skip, now + DURATION_24_HOURS);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--overwrite",
        "--file",
        &archive_path,
        &file_to_update,
        &file_to_skip,
        "--unstable",
        "--older-mtime-than",
        &reference_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(&archive_path, |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(seen, HashSet::from([file_to_update.clone()]));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--file",
        &archive_path,
        "--out-dir",
        &format!("{base_dir}/out"),
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content = fs::read_to_string(format!("{base_dir}/out/{file_to_update}")).unwrap();
    assert_eq!(updated_content, "updated content");
    assert!(
        fs::metadata(format!("{base_dir}/out/{file_to_skip}")).is_err(),
        "skip file should not have been extracted/added"
    );
}
