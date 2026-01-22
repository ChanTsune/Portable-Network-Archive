use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Verifies that updating an archive preserves the file entry order specified on the CLI.
///
/// Sets up three files, creates an initial archive, modifies the files to produce differing processing
/// durations, runs `pna experimental update` with a reordered list of files, and asserts the resulting
/// archive entries appear in the same order as the CLI arguments.
#[test]
fn update_preserves_cli_argument_order() {
    setup();
    let dir = "update_preserves_cli_argument_order";
    fs::create_dir_all(dir).unwrap();

    // Create initial files
    let file_a = format!("{dir}/a.txt");
    let file_b = format!("{dir}/b.txt");
    let file_c = format!("{dir}/c.txt");

    fs::write(&file_a, b"original a").unwrap();
    fs::write(&file_b, b"original b").unwrap();
    fs::write(&file_c, b"original c").unwrap();

    // Create initial archive with specific order: a, b, c
    let archive_path = format!("{dir}/ordered.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        &file_a,
        &file_b,
        &file_c,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify files with different sizes to affect parallel processing time
    fs::write(&file_a, vec![0u8; 1024 * 1024]).unwrap(); // 1MB - slow
    fs::write(&file_b, b"updated b - small").unwrap(); // small - fast
    fs::write(&file_c, vec![1u8; 100 * 1024]).unwrap(); // 100KB - medium

    // Update with specific order: b, a, c (different from original)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        &archive_path,
        &file_b,
        &file_a,
        &file_c,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        entry_names.push(entry.header().path().to_string());
    })
    .unwrap();

    // Update should preserve the order specified in CLI arguments: b, a, c
    assert_eq!(entry_names.len(), 3);
    assert!(
        entry_names[0].ends_with("b.txt"),
        "First entry should be b.txt, got: {}",
        entry_names[0]
    );
    assert!(
        entry_names[1].ends_with("a.txt"),
        "Second entry should be a.txt, got: {}",
        entry_names[1]
    );
    assert!(
        entry_names[2].ends_with("c.txt"),
        "Third entry should be c.txt, got: {}",
        entry_names[2]
    );
}

/// Verifies that updating an archive preserves the ordering of entries when multiple directories are provided.
///
/// Creates two directories (dir_a and dir_b), builds an initial archive containing both in that order,
/// mutates files to create differing update characteristics, runs `pna experimental update` with the
/// directories specified as dir_a then dir_b, and asserts that every entry originating from dir_a
/// appears before every entry originating from dir_b in the updated archive.
#[test]
fn update_preserves_multiple_directory_order() {
    setup();
    let dir = "update_preserves_multiple_directory_order";
    fs::create_dir_all(format!("{dir}/dir_a")).unwrap();
    fs::create_dir_all(format!("{dir}/dir_b")).unwrap();

    // Create initial files
    fs::write(format!("{dir}/dir_a/file.txt"), b"original a").unwrap();
    fs::write(format!("{dir}/dir_b/file.txt"), b"original b").unwrap();

    // Create initial archive
    let archive_path = format!("{dir}/multi_dir.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        &format!("{dir}/dir_a"),
        &format!("{dir}/dir_b"),
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify files: dir_a has large file (slow), dir_b has small files (fast)
    fs::write(format!("{dir}/dir_a/file.txt"), vec![0u8; 1024 * 1024]).unwrap();
    fs::write(format!("{dir}/dir_b/file.txt"), b"small").unwrap();
    fs::write(format!("{dir}/dir_b/extra.txt"), b"extra small").unwrap();

    // Update: dir_a first, then dir_b
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        &archive_path,
        &format!("{dir}/dir_a"),
        &format!("{dir}/dir_b"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        entry_names.push(entry.header().path().to_string());
    })
    .unwrap();

    let dir_a_indices: Vec<usize> = entry_names
        .iter()
        .enumerate()
        .filter(|(_, name)| name.contains("dir_a"))
        .map(|(i, _)| i)
        .collect();
    let dir_b_indices: Vec<usize> = entry_names
        .iter()
        .enumerate()
        .filter(|(_, name)| name.contains("dir_b"))
        .map(|(i, _)| i)
        .collect();

    assert!(!dir_a_indices.is_empty(), "Should have dir_a entries");
    assert!(!dir_b_indices.is_empty(), "Should have dir_b entries");

    let max_dir_a = *dir_a_indices.iter().max().unwrap();
    let min_dir_b = *dir_b_indices.iter().min().unwrap();
    assert!(
        max_dir_a < min_dir_b,
        "All dir_a entries (max index {}) should come before dir_b entries (min index {})",
        max_dir_a,
        min_dir_b
    );
}