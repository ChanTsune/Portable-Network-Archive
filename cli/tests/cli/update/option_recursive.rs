use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: An archive exists with initial files. A directory with nested subdirectories is added.
/// Action: Run `pna experimental update` with default behavior (recursive enabled).
/// Expectation: All files including those in subdirectories are added to the archive.
#[test]
fn update_with_recursive() {
    setup();

    let _ = fs::remove_dir_all("update_recursive");
    TestResources::extract_in("raw/", "update_recursive/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_recursive/archive.pna",
        "--overwrite",
        "update_recursive/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Add new directory structure with nested files
    fs::create_dir_all("update_recursive/in/raw/subdir/nested").unwrap();
    fs::write("update_recursive/in/raw/subdir/file1.txt", "file in subdir").unwrap();
    fs::write(
        "update_recursive/in/raw/subdir/nested/file2.txt",
        "file in nested",
    )
    .unwrap();

    // Update with default recursive behavior
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_recursive/archive.pna",
        "update_recursive/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_recursive/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Verify all files including nested ones are included
    assert!(
        seen.contains("update_recursive/in/raw/subdir/file1.txt"),
        "file in subdir should be included with recursive: {seen:?}"
    );
    assert!(
        seen.contains("update_recursive/in/raw/subdir/nested/file2.txt"),
        "file in nested dir should be included with recursive: {seen:?}"
    );
}

/// Precondition: An archive exists. A directory with nested files exists.
/// Action: Run `pna experimental update` with `--no-recursive` and `--keep-dir` on the directory.
/// Expectation: Only the directory entry is added (with --keep-dir), not the files inside.
#[test]
fn update_with_no_recursive_keep_dir() {
    setup();

    let _ = fs::remove_dir_all("update_no_recursive_keep_dir");

    // Create directory structure
    fs::create_dir_all("update_no_recursive_keep_dir/in/mydir").unwrap();
    fs::write(
        "update_no_recursive_keep_dir/in/mydir/file.txt",
        "file inside dir",
    )
    .unwrap();

    // Create initial archive with just a placeholder file
    fs::write(
        "update_no_recursive_keep_dir/in/placeholder.txt",
        "placeholder",
    )
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_no_recursive_keep_dir/archive.pna",
        "--overwrite",
        "update_no_recursive_keep_dir/in/placeholder.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update with --no-recursive and --keep-dir on the directory
    // With --no-recursive + --keep-dir, only the directory entry itself is added
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_no_recursive_keep_dir/archive.pna",
        "--no-recursive",
        "--keep-dir",
        "update_no_recursive_keep_dir/in/mydir/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_no_recursive_keep_dir/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // The directory entry should be added (with --keep-dir)
    assert!(
        seen.contains("update_no_recursive_keep_dir/in/mydir"),
        "directory entry should be included with --no-recursive --keep-dir: {seen:?}"
    );

    // But the file inside should NOT be added (because of --no-recursive)
    assert!(
        !seen.contains("update_no_recursive_keep_dir/in/mydir/file.txt"),
        "file inside directory should NOT be included with --no-recursive: {seen:?}"
    );
}

/// Precondition: An archive exists. A new nested directory structure is created.
/// Action: Run `pna experimental update` with `--recursive` flag explicitly.
/// Expectation: Behaves same as default, all nested files are added.
#[test]
fn update_with_explicit_recursive_flag() {
    setup();

    let _ = fs::remove_dir_all("update_explicit_recursive");
    TestResources::extract_in("raw/", "update_explicit_recursive/in/").unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_explicit_recursive/archive.pna",
        "--overwrite",
        "update_explicit_recursive/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Add new directory structure
    fs::create_dir_all("update_explicit_recursive/in/raw/deep/dir/structure").unwrap();
    fs::write(
        "update_explicit_recursive/in/raw/deep/dir/structure/deep_file.txt",
        "deep file content",
    )
    .unwrap();

    // Update with explicit --recursive flag
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_explicit_recursive/archive.pna",
        "--recursive",
        "update_explicit_recursive/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_explicit_recursive/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Deeply nested file should be included
    assert!(
        seen.contains("update_explicit_recursive/in/raw/deep/dir/structure/deep_file.txt"),
        "deeply nested file should be included with --recursive: {seen:?}"
    );
}

/// Precondition: An archive exists with files including those in subdirectories.
/// Action: Run `pna experimental update` with `--no-recursive` on a directory path.
/// Expectation: Existing entries from subdirectories remain in archive (not deleted), no new entries added.
#[test]
fn update_no_recursive_preserves_existing_entries() {
    setup();

    let _ = fs::remove_dir_all("update_no_recursive_preserve");

    // Create directory structure first
    fs::create_dir_all("update_no_recursive_preserve/in/subdir").unwrap();
    fs::write(
        "update_no_recursive_preserve/in/toplevel.txt",
        "original top",
    )
    .unwrap();
    fs::write(
        "update_no_recursive_preserve/in/subdir/nested.txt",
        "original nested",
    )
    .unwrap();

    // Create archive with all files (recursive by default)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_no_recursive_preserve/archive.pna",
        "--overwrite",
        "update_no_recursive_preserve/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify both files are in archive
    let mut initial_seen = HashSet::new();
    archive::for_each_entry("update_no_recursive_preserve/archive.pna", |entry| {
        initial_seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert!(
        initial_seen.contains("update_no_recursive_preserve/in/subdir/nested.txt"),
        "nested file should be in initial archive"
    );
    assert!(
        initial_seen.contains("update_no_recursive_preserve/in/toplevel.txt"),
        "toplevel file should be in initial archive"
    );

    // Add a new file in subdir
    fs::write(
        "update_no_recursive_preserve/in/subdir/new_nested.txt",
        "new nested file",
    )
    .unwrap();

    // Run update with --no-recursive and --keep-dir on the subdir
    // This should only add the subdir directory entry, not crawl into it
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_no_recursive_preserve/archive.pna",
        "--no-recursive",
        "--keep-dir",
        "update_no_recursive_preserve/in/subdir/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut final_seen = HashSet::new();
    archive::for_each_entry("update_no_recursive_preserve/archive.pna", |entry| {
        final_seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Original nested file should still exist (preserved from original archive)
    // because update without --sync doesn't remove entries
    assert!(
        final_seen.contains("update_no_recursive_preserve/in/subdir/nested.txt"),
        "existing nested file should be preserved: {final_seen:?}"
    );
    assert!(
        final_seen.contains("update_no_recursive_preserve/in/toplevel.txt"),
        "existing toplevel file should be preserved: {final_seen:?}"
    );

    // New nested file should NOT be added because --no-recursive doesn't recurse into subdir
    assert!(
        !final_seen.contains("update_no_recursive_preserve/in/subdir/new_nested.txt"),
        "new nested file should NOT be added with --no-recursive: {final_seen:?}"
    );
}

/// Precondition: An archive exists. Specific files are passed as arguments.
/// Action: Run `pna experimental update` with `--no-recursive` and individual file paths.
/// Expectation: Individual files are still processed (no directory recursion to skip).
#[test]
fn update_no_recursive_with_file_args() {
    setup();

    let _ = fs::remove_dir_all("update_no_recursive_files");

    // Create directory structure
    fs::create_dir_all("update_no_recursive_files/in/dir").unwrap();
    fs::write("update_no_recursive_files/in/file1.txt", "file1").unwrap();
    fs::write("update_no_recursive_files/in/file2.txt", "file2").unwrap();
    fs::write("update_no_recursive_files/in/dir/nested.txt", "nested").unwrap();

    // Create initial archive with all files
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_no_recursive_files/archive.pna",
        "--overwrite",
        "update_no_recursive_files/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify the files
    fs::write("update_no_recursive_files/in/file1.txt", "file1 updated").unwrap();
    fs::write("update_no_recursive_files/in/file2.txt", "file2 updated").unwrap();
    fs::write(
        "update_no_recursive_files/in/dir/nested.txt",
        "nested updated",
    )
    .unwrap();

    // Update with --no-recursive but passing individual file paths (not directories)
    // Individual files should still be processed since they're not directories to recurse into
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_no_recursive_files/archive.pna",
        "--no-recursive",
        "update_no_recursive_files/in/file1.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract and verify
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_no_recursive_files/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_no_recursive_files/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // file1.txt should be updated
    let file1_content = fs::read_to_string("update_no_recursive_files/out/file1.txt").unwrap();
    assert_eq!(
        file1_content, "file1 updated",
        "file1.txt should be updated"
    );

    // file2.txt should still have original content (not updated)
    let file2_content = fs::read_to_string("update_no_recursive_files/out/file2.txt").unwrap();
    assert_eq!(
        file2_content, "file2",
        "file2.txt should NOT be updated when not specified"
    );
}

/// Precondition: An archive exists with deeply nested files.
/// Action: Run `pna experimental update` with `--recursive` (default) then with `--no-recursive`.
/// Expectation: Recursive traverses all levels, non-recursive only processes the specified path.
#[test]
fn update_recursive_vs_no_recursive_comparison() {
    setup();

    let _ = fs::remove_dir_all("update_recursive_compare");

    // Create directory structure with multiple nesting levels
    fs::create_dir_all("update_recursive_compare/in/level1/level2/level3").unwrap();
    fs::write("update_recursive_compare/in/root.txt", "root").unwrap();
    fs::write("update_recursive_compare/in/level1/l1.txt", "level1").unwrap();
    fs::write("update_recursive_compare/in/level1/level2/l2.txt", "level2").unwrap();
    fs::write(
        "update_recursive_compare/in/level1/level2/level3/l3.txt",
        "level3",
    )
    .unwrap();

    // Create initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_recursive_compare/archive.pna",
        "--overwrite",
        "update_recursive_compare/in/root.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Update with --no-recursive on level1 directory - should only add level1 dir entry
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_recursive_compare/archive.pna",
        "--no-recursive",
        "--keep-dir",
        "update_recursive_compare/in/level1/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut after_no_recursive = HashSet::new();
    archive::for_each_entry("update_recursive_compare/archive.pna", |entry| {
        after_no_recursive.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Only root.txt and level1 directory should be present
    assert!(
        after_no_recursive.contains("update_recursive_compare/in/root.txt"),
        "root.txt should be present"
    );
    assert!(
        after_no_recursive.contains("update_recursive_compare/in/level1"),
        "level1 directory should be present with --keep-dir: {after_no_recursive:?}"
    );
    // Files inside level1 should NOT be present
    assert!(
        !after_no_recursive.contains("update_recursive_compare/in/level1/l1.txt"),
        "l1.txt should NOT be present with --no-recursive: {after_no_recursive:?}"
    );
    assert!(
        !after_no_recursive.contains("update_recursive_compare/in/level1/level2/l2.txt"),
        "l2.txt should NOT be present: {after_no_recursive:?}"
    );

    // Now update WITH --recursive on level1 directory
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_recursive_compare/archive.pna",
        "--recursive",
        "update_recursive_compare/in/level1/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut after_recursive = HashSet::new();
    archive::for_each_entry("update_recursive_compare/archive.pna", |entry| {
        after_recursive.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Now all files should be present
    assert!(
        after_recursive.contains("update_recursive_compare/in/level1/l1.txt"),
        "l1.txt should be present after recursive update: {after_recursive:?}"
    );
    assert!(
        after_recursive.contains("update_recursive_compare/in/level1/level2/l2.txt"),
        "l2.txt should be present after recursive update: {after_recursive:?}"
    );
    assert!(
        after_recursive.contains("update_recursive_compare/in/level1/level2/level3/l3.txt"),
        "l3.txt should be present after recursive update: {after_recursive:?}"
    );
}
