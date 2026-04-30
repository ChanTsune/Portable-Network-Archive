use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time};

/// Precondition: An archive contains a file.
/// Action: Create files with different ctimes, run `pna experimental update` with `--older-ctime`.
/// Expectation: Only files with ctime older than threshold are updated in the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn update_with_older_ctime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_older_ctime");
    fs::create_dir_all("update_older_ctime").unwrap();

    // Create initial file (will have older ctime)
    let file_to_update = "update_older_ctime/file_to_update.txt";
    let file_to_skip = "update_older_ctime/file_to_skip.txt";

    fs::write(file_to_update, "initial content").unwrap();

    // Check if creation time is available on this filesystem
    if fs::metadata(file_to_update).unwrap().created().is_err() {
        eprintln!("Skipping test: creation time (birth time) is not supported on this filesystem");
        return;
    }

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_older_ctime/archive.pna",
        "--overwrite",
        file_to_update,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update file_to_update content (keeps same ctime on most filesystems)
    fs::write(file_to_update, "updated content").unwrap();
    let file_to_update_ctime = fs::metadata(file_to_update).unwrap().created().unwrap();

    // Wait until we're in the next second to ensure threshold_file has a different second
    // This is necessary because --older-ctime uses seconds precision
    wait_until_next_second(file_to_update_ctime);

    // Create threshold file (now guaranteed to be in a later second)
    let threshold_file = "update_older_ctime/threshold.txt";
    fs::write(threshold_file, "threshold marker").unwrap();
    let threshold_ctime = fs::metadata(threshold_file).unwrap().created().unwrap();

    // Verify file_to_update has ctime < threshold (in seconds)
    let file_to_update_secs = file_to_update_ctime
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let threshold_secs = threshold_ctime
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if file_to_update_secs >= threshold_secs {
        eprintln!(
            "Skipping test: file_to_update ctime ({}) is not older than threshold ({}) in seconds",
            file_to_update_secs, threshold_secs
        );
        return;
    }

    // Wait until next second and create file_to_skip (will have newer ctime than threshold)
    wait_until_next_second(threshold_ctime);
    fs::write(file_to_skip, "skip content").unwrap();
    let file_to_skip_ctime = fs::metadata(file_to_skip).unwrap().created().unwrap();
    let file_to_skip_secs = file_to_skip_ctime
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Verify file_to_skip has ctime >= threshold (in seconds)
    if file_to_skip_secs < threshold_secs {
        eprintln!(
            "Skipping test: file_to_skip ctime ({}) is not newer than threshold ({}) in seconds",
            file_to_skip_secs, threshold_secs
        );
        return;
    }

    // Run update with --older-ctime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--older-ctime",
        &format!("@{}", threshold_secs),
        "-f",
        "update_older_ctime/archive.pna",
        file_to_update,
        file_to_skip,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents
    let mut seen = HashSet::new();
    archive::for_each_entry("update_older_ctime/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // file_to_update should be in archive (ctime <= threshold)
    assert!(
        seen.contains(file_to_update),
        "file_to_update should be in the archive: {file_to_update}"
    );

    // file_to_skip should NOT be in archive (ctime > threshold)
    assert!(
        !seen.contains(file_to_skip),
        "file_to_skip should NOT be added to archive: {file_to_skip}"
    );

    // Verify exactly 1 entry
    assert_eq!(
        seen.len(),
        1,
        "Expected exactly 1 entry, but found {}: {seen:?}",
        seen.len()
    );

    // Extract and verify content
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_older_ctime/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_older_ctime/out/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content =
        fs::read_to_string(format!("update_older_ctime/out/{file_to_update}")).unwrap();
    assert_eq!(
        updated_content, "updated content",
        "file_to_update should have the updated content"
    );

    // Verify file_to_skip was not extracted
    assert!(
        !std::path::Path::new(&format!("update_older_ctime/out/{file_to_skip}")).exists(),
        "file_to_skip should not exist in extracted output"
    );
}

/// Wait until the current second changes from the baseline second.
/// This is needed because the CLI uses seconds precision for timestamps.
fn wait_until_next_second(baseline: time::SystemTime) {
    let baseline_secs = baseline
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    loop {
        let now_secs = time::SystemTime::now()
            .duration_since(time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now_secs > baseline_secs {
            break;
        }
        thread::sleep(time::Duration::from_millis(10));
    }
}
