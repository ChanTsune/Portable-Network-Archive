use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, path::Path};

/// Precondition: An archive contains symlink entries alongside file entries.
/// Action: Run `pna experimental delete` to remove a symlink entry.
/// Expectation: The symlink entry is removed while other entries remain.
#[test]
fn delete_symlink_entry() {
    setup();

    // Create source directory structure with symlinks
    if Path::new("delete_symlink_entry").exists() {
        fs::remove_dir_all("delete_symlink_entry").unwrap();
    }
    fs::create_dir_all("delete_symlink_entry/in").unwrap();
    fs::write("delete_symlink_entry/in/target.txt", b"target content").unwrap();
    pna::fs::symlink(Path::new("target.txt"), "delete_symlink_entry/in/link.txt").unwrap();

    // Create archive (symlinks stored as symlinks by default)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_symlink_entry/archive.pna",
        "--overwrite",
        "delete_symlink_entry/in",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify symlink entry exists before deletion
    let mut has_symlink = false;
    archive::for_each_entry("delete_symlink_entry/archive.pna", |entry| {
        if entry.header().path().as_str() == "delete_symlink_entry/in/link.txt" {
            assert_eq!(
                entry.header().data_kind(),
                pna::DataKind::SymbolicLink,
                "link.txt should be a symlink entry"
            );
            has_symlink = true;
        }
    })
    .unwrap();
    assert!(
        has_symlink,
        "archive should contain symlink entry before deletion"
    );

    // Delete the symlink entry
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_symlink_entry/archive.pna",
        "delete_symlink_entry/in/link.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify results
    let mut seen = HashSet::new();
    archive::for_each_entry("delete_symlink_entry/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // The symlink entry should be deleted
    assert!(
        !seen.contains("delete_symlink_entry/in/link.txt"),
        "symlink entry should have been deleted"
    );

    // The target file should remain
    assert!(
        seen.contains("delete_symlink_entry/in/target.txt"),
        "target.txt should remain"
    );
}

/// Precondition: An archive contains a symlink pointing to a file, both stored as entries.
/// Action: Run `pna experimental delete` to remove the target file entry.
/// Expectation: The target file is removed while the symlink entry remains.
#[test]
fn delete_symlink_target_keeps_symlink() {
    setup();

    // Create source directory structure with symlinks
    if Path::new("delete_symlink_target").exists() {
        fs::remove_dir_all("delete_symlink_target").unwrap();
    }
    fs::create_dir_all("delete_symlink_target/in").unwrap();
    fs::write("delete_symlink_target/in/target.txt", b"target content").unwrap();
    pna::fs::symlink(Path::new("target.txt"), "delete_symlink_target/in/link.txt").unwrap();

    // Create archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_symlink_target/archive.pna",
        "--overwrite",
        "delete_symlink_target/in",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Delete the target file (not the symlink)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_symlink_target/archive.pna",
        "delete_symlink_target/in/target.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify results
    let mut seen_symlink = false;
    let mut seen_target = false;
    archive::for_each_entry("delete_symlink_target/archive.pna", |entry| {
        let path = entry.header().path().to_string();
        if path == "delete_symlink_target/in/link.txt" {
            assert_eq!(
                entry.header().data_kind(),
                pna::DataKind::SymbolicLink,
                "link.txt should still be a symlink"
            );
            seen_symlink = true;
        }
        if path == "delete_symlink_target/in/target.txt" {
            seen_target = true;
        }
    })
    .unwrap();

    assert!(!seen_target, "target.txt should have been deleted");
    assert!(
        seen_symlink,
        "symlink entry should remain after deleting target"
    );
}

/// Precondition: An archive contains directory symlinks alongside regular directories.
/// Action: Run `pna experimental delete` to remove a directory symlink entry.
/// Expectation: The directory symlink is removed while the target directory and its contents remain.
#[test]
fn delete_directory_symlink_entry() {
    setup();

    // Create source directory structure with directory symlink
    if Path::new("delete_dir_symlink").exists() {
        fs::remove_dir_all("delete_dir_symlink").unwrap();
    }
    fs::create_dir_all("delete_dir_symlink/in/realdir").unwrap();
    fs::write("delete_dir_symlink/in/realdir/file.txt", b"content").unwrap();
    pna::fs::symlink(Path::new("realdir"), "delete_dir_symlink/in/linkdir").unwrap();

    // Create archive with directory entries
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_dir_symlink/archive.pna",
        "--overwrite",
        "--keep-dir",
        "delete_dir_symlink/in",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Delete the directory symlink
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_dir_symlink/archive.pna",
        "delete_dir_symlink/in/linkdir",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify results
    let mut seen = HashSet::new();
    archive::for_each_entry("delete_dir_symlink/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // The directory symlink should be deleted
    assert!(
        !seen.contains("delete_dir_symlink/in/linkdir"),
        "directory symlink should have been deleted"
    );

    // The real directory and its contents should remain
    for required in [
        "delete_dir_symlink/in",
        "delete_dir_symlink/in/realdir",
        "delete_dir_symlink/in/realdir/file.txt",
    ] {
        assert!(
            seen.contains(required),
            "required entry missing: {required}"
        );
    }
}
