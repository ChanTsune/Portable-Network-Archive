use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{collections::HashSet, fs, thread, time};

/// Precondition: Create an archive with file_to_update, then create reference, update files, and new file.
/// Action: Run `pna experimental update` with `--newer-ctime-than reference.txt`.
/// Expectation: Files with ctime >= reference.txt are updated or added to the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn update_with_newer_ctime_than() {
    setup();
    let reference_file = "update_newer_ctime_than/reference.txt";
    let file_to_update = "update_newer_ctime_than/file_to_update.txt";
    let file_to_add = "update_newer_ctime_than/file_to_add.txt";

    // Create directory
    fs::create_dir_all("update_newer_ctime_than").unwrap();

    // 1. Create the initial file and archive it.
    fs::write(file_to_update, "initial content").unwrap();

    // Check if creation time is available on this filesystem
    if fs::metadata(file_to_update).unwrap().created().is_err() {
        eprintln!("Skipping test: creation time (birth time) is not supported on this filesystem");
        return;
    }
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_newer_ctime_than/test.pna",
        "--overwrite",
        file_to_update,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // 2. Wait and create a reference file to set a timestamp benchmark.
    thread::sleep(time::Duration::from_millis(10));
    fs::write(reference_file, "time reference").unwrap();

    // 3. Wait, then recreate the existing file (to ensure both ctime/birth time move
    //    forward) and create the new file so that both are newer than the reference.
    thread::sleep(time::Duration::from_millis(10));
    fs::remove_file(file_to_update).unwrap();
    fs::write(file_to_update, "updated content").unwrap();
    fs::write(file_to_add, "new file content").unwrap();

    // 4. Run the update command, targeting the files to be updated/added,
    //    filtered by the ctime of the reference file.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--file",
        "update_newer_ctime_than/test.pna",
        file_to_update,
        file_to_add,
        "--unstable",
        "--newer-ctime-than",
        reference_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // 5. Verify archive contents
    let mut seen = HashSet::new();
    archive::for_each_entry("update_newer_ctime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // file_to_update should be present (updated because ctime > reference)
    assert!(
        seen.contains(file_to_update),
        "updated file should be in archive: {file_to_update}"
    );

    // file_to_add should be present (added because ctime > reference)
    assert!(
        seen.contains(file_to_add),
        "new file should be added: {file_to_add}"
    );

    // Verify that exactly two entries exist
    assert_eq!(
        seen.len(),
        2,
        "Expected exactly 2 entries, but found {}: {seen:?}",
        seen.len()
    );

    // 6. Extract and verify the content of the updated file.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--file",
        "update_newer_ctime_than/test.pna",
        "--out-dir",
        "update_newer_ctime_than/out",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content = fs::read_to_string(
        "update_newer_ctime_than/out/update_newer_ctime_than/file_to_update.txt",
    )
    .unwrap();
    assert_eq!(
        updated_content, "updated content",
        "The updated file did not contain the correct content"
    );
}
