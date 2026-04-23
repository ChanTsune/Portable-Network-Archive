use crate::utils::{
    archive, setup,
    time::{confirm_time_older_than, wait_until_time_newer_than},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time::Duration};

/// Precondition: An archive exists with files to update, and the source tree contains a reference file and files with varying creation times.
/// Action: Run `pna experimental update` with `--older-ctime-than` pointing to the reference file.
/// Expectation: Only files whose creation time is older than the reference file are processed in the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn update_with_older_ctime_than() {
    setup();
    let base_dir = "update_older_ctime_than";
    let archive_path = format!("{base_dir}/test.pna");
    let file_to_update = format!("{base_dir}/file_to_update.txt");
    let file_to_skip = format!("{base_dir}/file_to_skip.txt");
    let reference_file = format!("{base_dir}/reference.txt");

    fs::create_dir_all(base_dir).unwrap();
    fs::write(&file_to_update, "initial content").unwrap();

    if fs::metadata(&file_to_update).unwrap().created().is_err() {
        eprintln!("Skipping test: creation time (birth time) not supported on this filesystem");
        return;
    }

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

    // Recreate the file with updated content *before* creating the reference so its ctime stays older.
    thread::sleep(Duration::from_millis(10));
    fs::write(&file_to_update, "updated content").unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(&reference_file, "reference marker").unwrap();
    let reference_ctime = fs::metadata(&reference_file).unwrap().created().unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(&file_to_skip, "skip content").unwrap();

    if !confirm_time_older_than(&file_to_update, reference_ctime, |m| m.created().ok())
        || !wait_until_time_newer_than(&file_to_skip, reference_ctime, |m| m.created().ok())
    {
        eprintln!(
            "Skipping test: unable to create deterministic creation times on this filesystem"
        );
        return;
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--file",
        &archive_path,
        &file_to_update,
        &file_to_skip,
        "--unstable",
        "--older-ctime-than",
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

    assert!(
        !seen.contains(&file_to_skip),
        "file newer than reference should not have been added: {file_to_skip}"
    );

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
