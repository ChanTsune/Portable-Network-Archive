use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: An archive contains directory entries alongside file entries.
/// Action: Run `pna experimental delete` to remove a directory entry.
/// Expectation: The directory entry is removed while file entries within remain.
#[test]
fn delete_directory_entry() {
    setup();

    // Create source directory structure
    let base = "delete_directory_entry";
    if std::path::Path::new(base).exists() {
        fs::remove_dir_all(base).unwrap();
    }
    fs::create_dir_all(format!("{base}/in/subdir")).unwrap();
    fs::write(format!("{base}/in/root.txt"), b"root content").unwrap();
    fs::write(format!("{base}/in/subdir/nested.txt"), b"nested content").unwrap();

    // Create archive with directory entries
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/archive.pna"),
        "--overwrite",
        "--keep-dir",
        &format!("{base}/in"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify directory entries exist before deletion
    let mut before = HashSet::new();
    archive::for_each_entry(format!("{base}/archive.pna"), |entry| {
        before.insert((
            entry.header().path().to_string(),
            entry.header().data_kind(),
        ));
    })
    .unwrap();
    assert!(
        before.contains(&(format!("{base}/in"), pna::DataKind::Directory)),
        "archive should contain directory entry before deletion"
    );
    assert!(
        before.contains(&(format!("{base}/in/subdir"), pna::DataKind::Directory)),
        "archive should contain subdir entry before deletion"
    );

    // Delete only the subdir directory entry
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        &format!("{base}/archive.pna"),
        &format!("{base}/in/subdir"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify results
    let mut seen = HashSet::new();
    archive::for_each_entry(format!("{base}/archive.pna"), |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // The subdir directory entry should be deleted
    assert!(
        !seen.contains(&format!("{base}/in/subdir")),
        "subdir directory entry should have been deleted"
    );

    // Other entries should remain
    for required in [
        format!("{base}/in"),
        format!("{base}/in/root.txt"),
        format!("{base}/in/subdir/nested.txt"),
    ] {
        assert!(
            seen.take(&required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

/// Precondition: An archive contains directory entries alongside file entries.
/// Action: Run `pna experimental delete` with a glob pattern matching a directory and its contents.
/// Expectation: Both the directory entry and all entries within are removed.
#[test]
fn delete_directory_and_contents() {
    setup();

    // Create source directory structure
    let base = "delete_directory_and_contents";
    if std::path::Path::new(base).exists() {
        fs::remove_dir_all(base).unwrap();
    }
    fs::create_dir_all(format!("{base}/in/keep")).unwrap();
    fs::create_dir_all(format!("{base}/in/remove")).unwrap();
    fs::write(format!("{base}/in/keep/keep.txt"), b"keep").unwrap();
    fs::write(format!("{base}/in/remove/remove.txt"), b"remove").unwrap();

    // Create archive with directory entries
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/archive.pna"),
        "--overwrite",
        "--keep-dir",
        &format!("{base}/in"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Delete the 'remove' directory and all its contents
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        &format!("{base}/archive.pna"),
        &format!("{base}/in/remove"),
        &format!("{base}/in/remove/**"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify results
    let mut seen = HashSet::new();
    archive::for_each_entry(format!("{base}/archive.pna"), |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // The 'remove' directory and its contents should be deleted
    assert!(
        !seen.contains(&format!("{base}/in/remove")),
        "remove directory entry should have been deleted"
    );
    assert!(
        !seen.contains(&format!("{base}/in/remove/remove.txt")),
        "remove/remove.txt should have been deleted"
    );

    // The 'keep' directory and its contents should remain
    for required in [
        format!("{base}/in"),
        format!("{base}/in/keep"),
        format!("{base}/in/keep/keep.txt"),
    ] {
        assert!(
            seen.take(&required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
