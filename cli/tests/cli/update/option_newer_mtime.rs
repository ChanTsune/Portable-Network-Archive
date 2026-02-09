use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, io::prelude::*, time};

const DURATION_24_HOURS: time::Duration = time::Duration::from_secs(24 * 60 * 60);

/// Precondition: An archive contains multiple files.
/// Action: Modify files with different mtimes, run `pna experimental update` with `--newer-mtime`.
/// Expectation: Only files with mtime newer than threshold are updated in the archive.
#[test]
fn update_with_newer_mtime() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_newer_mtime");
    TestResources::extract_in("raw/", "update_newer_mtime/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_newer_mtime/archive.pna",
        "--overwrite",
        "update_newer_mtime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Record the threshold time
    let threshold = time::SystemTime::now();

    // Modify empty.txt with OLDER mtime (should NOT be updated)
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_newer_mtime/in/raw/empty.txt")
        .unwrap();
    file.write_all(b"modified but older mtime").unwrap();
    file.set_modified(threshold - DURATION_24_HOURS).unwrap();

    // Modify text.txt with NEWER mtime (should be updated)
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_newer_mtime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated content").unwrap();
    file.set_modified(threshold + DURATION_24_HOURS).unwrap();

    // Run update with --newer-mtime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--unstable",
        "--newer-mtime",
        &format!(
            "@{}",
            threshold
                .duration_since(time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ),
        "-f",
        "update_newer_mtime/archive.pna",
        "update_newer_mtime/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents
    let mut seen = HashSet::new();
    archive::for_each_entry("update_newer_mtime/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Both files should be in archive
    assert!(
        seen.iter().any(|p| p.ends_with("raw/empty.txt")),
        "empty.txt should be in the archive"
    );
    assert!(
        seen.iter().any(|p| p.ends_with("raw/text.txt")),
        "text.txt should be in the archive"
    );

    // Extract and verify content
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_newer_mtime/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_newer_mtime/out/",
        "--keep-timestamp",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // empty.txt should have ORIGINAL content (was not updated due to older mtime)
    let empty_content = fs::read_to_string("update_newer_mtime/out/raw/empty.txt").unwrap();
    assert!(
        empty_content.is_empty(),
        "empty.txt should retain original empty content, got: {empty_content:?}"
    );

    // text.txt should have UPDATED content (was updated due to newer mtime)
    let text_content = fs::read_to_string("update_newer_mtime/out/raw/text.txt").unwrap();
    assert_eq!(
        text_content, "updated content",
        "text.txt should have updated content"
    );
}
