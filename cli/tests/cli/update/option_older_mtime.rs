use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, thread, time};

/// Precondition: An archive contains a file.
/// Action: Create files with different mtimes, run `pna experimental update` with `--older-mtime`.
/// Expectation: Only files with mtime older than threshold are added to the archive.
#[test]
fn update_with_older_mtime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_older_mtime");
    fs::create_dir_all("update_older_mtime").unwrap();

    // Create initial file
    let file_to_update = "update_older_mtime/file_to_update.txt";
    let file_to_skip = "update_older_mtime/file_to_skip.txt";

    fs::write(file_to_update, "initial content").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_older_mtime/archive.pna",
        "--overwrite",
        file_to_update,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Wait, then update file_to_update (will have older mtime after we set it)
    thread::sleep(time::Duration::from_millis(10));
    fs::write(file_to_update, "updated content").unwrap();

    // Record threshold time
    let threshold = time::SystemTime::now();

    // Set file_to_update mtime to BEFORE threshold
    let older_mtime = threshold - time::Duration::from_secs(24 * 60 * 60);
    let file = fs::File::options()
        .write(true)
        .open(file_to_update)
        .unwrap();
    file.set_modified(older_mtime).unwrap();

    // Wait, then create file_to_skip with NEWER mtime
    thread::sleep(time::Duration::from_millis(10));
    fs::write(file_to_skip, "skip content").unwrap();
    // file_to_skip naturally has mtime > threshold

    // Run update with --older-mtime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--older-mtime",
        &format!(
            "@{}",
            threshold
                .duration_since(time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ),
        "-f",
        "update_older_mtime/archive.pna",
        file_to_update,
        file_to_skip,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents
    let mut seen = HashSet::new();
    archive::for_each_entry("update_older_mtime/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // file_to_update should be in archive (mtime < threshold)
    assert!(
        seen.contains(file_to_update),
        "file_to_update should be in the archive: {file_to_update}"
    );

    // file_to_skip should NOT be in archive (mtime > threshold)
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

    // Extract and verify content - file_to_update should have updated content
    // (The --older-mtime filter allowed it through, and it was newer than archive entry)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_older_mtime/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_older_mtime/out/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content =
        fs::read_to_string(format!("update_older_mtime/out/{file_to_update}")).unwrap();
    assert_eq!(
        updated_content, "updated content",
        "file_to_update should have the updated content"
    );

    // Verify file_to_skip was not extracted
    assert!(
        !std::path::Path::new(&format!("update_older_mtime/out/{file_to_skip}")).exists(),
        "file_to_skip should not exist in extracted output"
    );
}
