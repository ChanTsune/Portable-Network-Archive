use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive exists with entries, files of varying sizes exist for update.
/// Action: Run `pna experimental update` with files in a specific order.
/// Expectation: Under append-only update semantics (`--sync` disabled), the
///   original entries are preserved and the newly appended entries follow CLI
///   argument order.
#[test]
fn update_preserves_cli_argument_order() {
    setup();
    let dir = "update_preserves_cli_argument_order";
    let _ = fs::remove_dir_all(dir);
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
        "-f",
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

    // Append-only semantics: original 3 entries (a, b, c in creation order)
    // remain, then 3 appended entries follow the CLI argument order (b, a, c).
    assert_eq!(entry_names.len(), 6);
    assert!(entry_names[0].ends_with("a.txt"));
    assert!(entry_names[1].ends_with("b.txt"));
    assert!(entry_names[2].ends_with("c.txt"));
    assert!(
        entry_names[3].ends_with("b.txt"),
        "First appended entry should be b.txt, got: {}",
        entry_names[3]
    );
    assert!(
        entry_names[4].ends_with("a.txt"),
        "Second appended entry should be a.txt, got: {}",
        entry_names[4]
    );
    assert!(
        entry_names[5].ends_with("c.txt"),
        "Third appended entry should be c.txt, got: {}",
        entry_names[5]
    );
}

/// Precondition: An archive exists, multiple directories with files exist.
/// Action: Run `pna experimental update` with multiple directory arguments.
/// Expectation: Within the appended portion of the archive (append-only `-u`
///   semantics), entries from the first CLI directory argument appear before
///   the second.
#[test]
fn update_preserves_multiple_directory_order() {
    setup();
    let dir = "update_preserves_multiple_directory_order";
    let _ = fs::remove_dir_all(dir);
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
        "-f",
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

    // Under append-only update semantics, the original archive entries remain
    // followed by appended entries. Verify ordering only within the appended
    // portion: entries from the first CLI argument (dir_a) must precede those
    // from the second (dir_b).
    let original_count = entry_names
        .iter()
        .position(|name| name.contains("extra.txt"))
        .unwrap_or(entry_names.len())
        .min(
            // Heuristic: original archive has 4 entries (dir_a/, dir_a/file.txt,
            // dir_b/, dir_b/file.txt). The newly-introduced extra.txt cannot
            // appear before the appended portion.
            4,
        );
    let appended = &entry_names[original_count..];

    let dir_a_max = appended
        .iter()
        .enumerate()
        .filter(|(_, name)| name.contains("dir_a"))
        .map(|(i, _)| i)
        .max();
    let dir_b_min = appended
        .iter()
        .enumerate()
        .filter(|(_, name)| name.contains("dir_b"))
        .map(|(i, _)| i)
        .min();

    let dir_a_max = dir_a_max.expect("appended portion should contain dir_a entries");
    let dir_b_min = dir_b_min.expect("appended portion should contain dir_b entries");
    assert!(
        dir_a_max < dir_b_min,
        "Appended dir_a entries (max index {dir_a_max}) should precede dir_b entries (min index {dir_b_min}). full order: {entry_names:?}",
    );
}
