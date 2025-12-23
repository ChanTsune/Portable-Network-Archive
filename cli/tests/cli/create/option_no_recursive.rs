use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: A directory contains files and subdirectories.
/// Action: Run `pna create` with `--no-recursive`.
/// Expectation: The archive contains only the top-level directory entry, not its contents.
#[test]
fn no_recursive() {
    setup();

    let _ = fs::remove_dir_all("no_recursive");
    fs::create_dir_all("no_recursive/in/subdir").unwrap();

    fs::write("no_recursive/in/file.txt", "content").unwrap();
    fs::write("no_recursive/in/subdir/nested.txt", "nested content").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "no_recursive/no_recursive.pna",
        "--overwrite",
        "--keep-dir",
        "--no-recursive",
        "no_recursive/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("no_recursive/no_recursive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // With --no-recursive, only the top-level directory should be included.
    assert!(
        seen.contains("no_recursive/in"),
        "top-level directory should be included"
    );
    assert!(
        !seen.contains("no_recursive/in/file.txt"),
        "file inside directory should NOT be included"
    );
    assert!(
        !seen.contains("no_recursive/in/subdir"),
        "subdirectory should NOT be included"
    );
    assert!(
        !seen.contains("no_recursive/in/subdir/nested.txt"),
        "nested file should NOT be included"
    );
    assert_eq!(
        seen.len(),
        1,
        "Expected exactly 1 entry (top-level directory only), but found {}: {seen:?}",
        seen.len()
    );
}
