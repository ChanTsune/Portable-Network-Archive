use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time};

/// Precondition: An archive contains a file.
/// Action: Recreate files with different ctimes, run `pna experimental update` with `--newer-ctime`.
/// Expectation: Only files with ctime newer than threshold are updated in the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn update_with_newer_ctime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_newer_ctime");
    fs::create_dir_all("update_newer_ctime").unwrap();

    // Create initial file
    let file_to_keep = "update_newer_ctime/file_to_keep.txt";
    let file_to_update = "update_newer_ctime/file_to_update.txt";

    fs::write(file_to_keep, "original content").unwrap();

    // Check if creation time is available on this filesystem
    if fs::metadata(file_to_keep).unwrap().created().is_err() {
        eprintln!("Skipping test: creation time (birth time) is not supported on this filesystem");
        return;
    }

    // Create initial archive with file_to_keep
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_newer_ctime/archive.pna",
        "--overwrite",
        file_to_keep,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Wait and record threshold time
    thread::sleep(time::Duration::from_millis(10));
    let threshold_file = "update_newer_ctime/threshold.txt";
    fs::write(threshold_file, "threshold marker").unwrap();
    let threshold_ctime = fs::metadata(threshold_file).unwrap().created().unwrap();

    // Wait, then create a new file that should be added (newer ctime)
    thread::sleep(time::Duration::from_millis(10));
    fs::write(file_to_update, "new content").unwrap();

    // Wait until the new file has ctime after threshold
    if !wait_until_ctime_after(file_to_update, threshold_ctime) {
        eprintln!(
            "Skipping test: creation time did not advance beyond threshold on this filesystem"
        );
        return;
    }

    // Run update with --newer-ctime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--newer-ctime",
        &format!(
            "@{}",
            threshold_ctime
                .duration_since(time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ),
        "-f",
        "update_newer_ctime/archive.pna",
        file_to_keep,
        file_to_update,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents
    let mut seen = HashSet::new();
    archive::for_each_entry("update_newer_ctime/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // file_to_keep should be in archive (from initial creation)
    assert!(
        seen.contains(file_to_keep),
        "file_to_keep should be in the archive: {file_to_keep}"
    );

    // file_to_update should be in archive (added because ctime > threshold)
    assert!(
        seen.contains(file_to_update),
        "file_to_update should be added to archive: {file_to_update}"
    );

    // Verify exactly 2 entries
    assert_eq!(
        seen.len(),
        2,
        "Expected exactly 2 entries, but found {}: {seen:?}",
        seen.len()
    );

    // Extract and verify content
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_newer_ctime/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_newer_ctime/out/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content =
        fs::read_to_string(format!("update_newer_ctime/out/{file_to_update}")).unwrap();
    assert_eq!(
        updated_content, "new content",
        "file_to_update should have the new content"
    );
}

fn wait_until_ctime_after(path: &str, baseline: time::SystemTime) -> bool {
    const MAX_ATTEMPTS: usize = 200;
    const SLEEP_MS: u64 = 10;
    for _ in 0..MAX_ATTEMPTS {
        if fs::metadata(path)
            .ok()
            .and_then(|meta| meta.created().ok())
            .map(|ctime| ctime > baseline)
            .unwrap_or(false)
        {
            return true;
        }
        thread::sleep(time::Duration::from_millis(SLEEP_MS));
    }
    false
}
