use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive exists and multiple files of varying sizes exist.
/// Action: Run `pna append` with files in a specific order.
/// Expectation: Appended entries appear in the same order as CLI arguments.
#[test]
fn append_preserves_cli_argument_order() {
    setup();
    let dir = "append_preserves_cli_argument_order";
    fs::create_dir_all(dir).unwrap();

    // Create initial archive with one file
    let initial_file = format!("{dir}/initial.txt");
    fs::write(&initial_file, b"initial content").unwrap();

    let archive_path = format!("{dir}/ordered.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        &initial_file,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Create files with different sizes to affect parallel processing time
    let large_file = format!("{dir}/large.bin");
    let small_file = format!("{dir}/small.txt");
    let medium_file = format!("{dir}/medium.dat");

    fs::write(&large_file, vec![0u8; 1024 * 1024]).unwrap();
    fs::write(&small_file, b"small file").unwrap();
    fs::write(&medium_file, vec![1u8; 100 * 1024]).unwrap();

    // Append with specific order: small, large, medium
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        &archive_path,
        &small_file,
        &large_file,
        &medium_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        entry_names.push(entry.header().path().to_string());
    })
    .unwrap();

    // First entry is the initial file, followed by appended files in order
    assert_eq!(entry_names.len(), 4);
    assert!(
        entry_names[0].ends_with("initial.txt"),
        "First entry should be initial.txt, got: {}",
        entry_names[0]
    );
    assert!(
        entry_names[1].ends_with("small.txt"),
        "Second entry should be small.txt, got: {}",
        entry_names[1]
    );
    assert!(
        entry_names[2].ends_with("large.bin"),
        "Third entry should be large.bin, got: {}",
        entry_names[2]
    );
    assert!(
        entry_names[3].ends_with("medium.dat"),
        "Fourth entry should be medium.dat, got: {}",
        entry_names[3]
    );
}

/// Precondition: An archive exists and multiple directories with files exist.
/// Action: Run `pna append` with multiple directory arguments.
/// Expectation: Entries from first argument appear before second argument.
#[test]
fn append_preserves_multiple_directory_order() {
    setup();
    let dir = "append_preserves_multiple_directory_order";
    fs::create_dir_all(format!("{dir}/dir_a")).unwrap();
    fs::create_dir_all(format!("{dir}/dir_b")).unwrap();

    // Create initial archive
    let initial_file = format!("{dir}/initial.txt");
    fs::write(&initial_file, b"initial").unwrap();

    let archive_path = format!("{dir}/multi_dir.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        &initial_file,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // dir_a has a large file (slow), dir_b has small files (fast)
    fs::write(format!("{dir}/dir_a/large.bin"), vec![0u8; 1024 * 1024]).unwrap();
    fs::write(format!("{dir}/dir_b/small1.txt"), b"small1").unwrap();
    fs::write(format!("{dir}/dir_b/small2.txt"), b"small2").unwrap();

    // Append: dir_a first, then dir_b
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
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

    // Find indices of dir_a and dir_b entries (excluding initial.txt)
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

    // All dir_a entries should come before all dir_b entries
    let max_dir_a = *dir_a_indices.iter().max().unwrap();
    let min_dir_b = *dir_b_indices.iter().min().unwrap();
    assert!(
        max_dir_a < min_dir_b,
        "All dir_a entries (max index {}) should come before dir_b entries (min index {})",
        max_dir_a,
        min_dir_b
    );
}

/// Precondition: A solid archive exists and multiple files of varying sizes exist.
/// Action: Run `pna append` with files in a specific order.
/// Expectation: Appended entries appear in the same order as CLI arguments.
#[test]
fn append_solid_preserves_cli_argument_order() {
    setup();
    let dir = "append_solid_preserves_cli_argument_order";
    fs::create_dir_all(dir).unwrap();

    // Create initial solid archive with one file
    let initial_file = format!("{dir}/initial.txt");
    fs::write(&initial_file, b"initial content").unwrap();

    let archive_path = format!("{dir}/ordered_solid.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        "--solid",
        &initial_file,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let large_file = format!("{dir}/large.bin");
    let small_file = format!("{dir}/small.txt");
    let medium_file = format!("{dir}/medium.dat");

    fs::write(&large_file, vec![0u8; 1024 * 1024]).unwrap();
    fs::write(&small_file, b"small file").unwrap();
    fs::write(&medium_file, vec![1u8; 100 * 1024]).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        &archive_path,
        &small_file,
        &large_file,
        &medium_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        entry_names.push(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(entry_names.len(), 4);
    assert!(
        entry_names[0].ends_with("initial.txt"),
        "First entry should be initial.txt, got: {}",
        entry_names[0]
    );
    assert!(
        entry_names[1].ends_with("small.txt"),
        "Second entry should be small.txt, got: {}",
        entry_names[1]
    );
    assert!(
        entry_names[2].ends_with("large.bin"),
        "Third entry should be large.bin, got: {}",
        entry_names[2]
    );
    assert!(
        entry_names[3].ends_with("medium.dat"),
        "Fourth entry should be medium.dat, got: {}",
        entry_names[3]
    );
}
