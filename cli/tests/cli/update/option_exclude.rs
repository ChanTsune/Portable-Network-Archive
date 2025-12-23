use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, io::prelude::*, time};

const DURATION_24_HOURS: time::Duration = time::Duration::from_secs(24 * 60 * 60);

/// Precondition: An archive contains multiple files.
/// Action: Modify files, run `pna experimental update` with `--exclude` for one file.
/// Expectation: Excluded file retains original content; non-excluded files are updated.
#[test]
fn update_with_exclude() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_with_exclude");
    TestResources::extract_in("raw/", "update_with_exclude/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_with_exclude/archive.pna",
        "--overwrite",
        "update_with_exclude/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify empty.txt (will be excluded)
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_exclude/in/raw/empty.txt")
        .unwrap();
    file.write_all(b"modified but excluded").unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Modify text.txt (will be updated)
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_with_exclude/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated content").unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Run update with --exclude
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_with_exclude/archive.pna",
        "update_with_exclude/in/",
        "--keep-timestamp",
        "--exclude",
        "update_with_exclude/in/raw/empty.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents
    let mut seen = HashSet::new();
    archive::for_each_entry("update_with_exclude/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Both files should still be in the archive
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
        "update_with_exclude/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_with_exclude/out/",
        "--keep-timestamp",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Excluded file should have original content (empty)
    let excluded_content = fs::read_to_string("update_with_exclude/out/raw/empty.txt").unwrap();
    assert!(
        excluded_content.is_empty(),
        "excluded file should retain original empty content, got: {excluded_content:?}"
    );

    // Non-excluded file should have updated content
    let updated_content = fs::read_to_string("update_with_exclude/out/raw/text.txt").unwrap();
    assert_eq!(
        updated_content, "updated content",
        "non-excluded file should have updated content"
    );
}
