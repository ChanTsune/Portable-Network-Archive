use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Verifies that `pna create` preserves the order of files given as CLI arguments.
///
/// Creates three files of different sizes, runs `pna create` with those files in a specific
/// argument order, and asserts that the resulting archive contains entries in the same order.
///
/// # Examples
///
/// ```rust
/// // Create files and run the CLI to produce an archive whose entries follow the CLI order.
/// let dir = "create_preserves_cli_argument_order_example";
/// std::fs::create_dir_all(dir).unwrap();
/// let small = format!("{}/small.txt", dir);
/// let large = format!("{}/large.bin", dir);
/// std::fs::write(&small, b"small").unwrap();
/// std::fs::write(&large, vec![0u8; 1024]).unwrap();
///
/// cli::Cli::try_parse_from([
///     "pna", "--quiet", "c", &format!("{}/out.pna", dir), "--overwrite", &small, &large, "--unstable",
/// ]).unwrap().execute().unwrap();
///
/// let mut names = Vec::new();
/// archive::for_each_entry(&format!("{}/out.pna", dir), |e| names.push(e.header().path().to_string())).unwrap();
/// assert!(names[0].ends_with("small.txt") && names[1].ends_with("large.bin"));
/// ```
#[test]
fn create_preserves_cli_argument_order() {
    setup();
    let dir = "create_preserves_cli_argument_order";
    fs::create_dir_all(dir).unwrap();

    // Create files with different sizes to affect parallel processing time
    // Large file processes slower, small file processes faster
    let large_file = format!("{dir}/large.bin");
    let small_file = format!("{dir}/small.txt");
    let medium_file = format!("{dir}/medium.dat");

    // Create files: large (1MB), small (10 bytes), medium (100KB)
    fs::write(&large_file, vec![0u8; 1024 * 1024]).unwrap();
    fs::write(&small_file, b"small file").unwrap();
    fs::write(&medium_file, vec![1u8; 100 * 1024]).unwrap();

    let archive_path = format!("{dir}/ordered.pna");

    // Create archive with specific order: small, large, medium
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        &small_file,
        &large_file,
        &medium_file,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        entry_names.push(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(entry_names.len(), 3);
    assert!(
        entry_names[0].ends_with("small.txt"),
        "First entry should be small.txt, got: {}",
        entry_names[0]
    );
    assert!(
        entry_names[1].ends_with("large.bin"),
        "Second entry should be large.bin, got: {}",
        entry_names[1]
    );
    assert!(
        entry_names[2].ends_with("medium.dat"),
        "Third entry should be medium.dat, got: {}",
        entry_names[2]
    );
}

/// Precondition: A directory with multiple files exists.
/// Action: Run `pna create` on the directory.
/// Expectation: Entries appear in consistent walkdir traversal order.
#[test]
fn create_preserves_walkdir_order() {
    setup();
    let dir = "create_preserves_walkdir_order";
    fs::create_dir_all(format!("{dir}/input/aaa")).unwrap();
    fs::create_dir_all(format!("{dir}/input/bbb")).unwrap();

    // Create files with different sizes in nested directories
    fs::write(format!("{dir}/input/01_first.txt"), b"first").unwrap();
    fs::write(
        format!("{dir}/input/aaa/02_in_aaa.txt"),
        vec![0u8; 500 * 1024],
    )
    .unwrap();
    fs::write(format!("{dir}/input/bbb/03_in_bbb.txt"), b"in bbb").unwrap();
    fs::write(format!("{dir}/input/04_last.txt"), vec![1u8; 200 * 1024]).unwrap();

    let archive_path = format!("{dir}/walkdir_order.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        &format!("{dir}/input"),
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify entries follow walkdir depth-first order
    let mut entry_names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        entry_names.push(entry.header().path().to_string());
    })
    .unwrap();

    // walkdir traverses depth-first, so directory entries come before their contents
    // Exact order depends on walkdir implementation, but should be consistent
    assert!(
        entry_names.len() >= 4,
        "Should have at least 4 file entries"
    );

    // Verify that the order is consistent (not randomized by parallel processing)
    // Run the same command again and verify same order
    let archive_path2 = format!("{dir}/walkdir_order2.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path2,
        "--overwrite",
        &format!("{dir}/input"),
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_names2 = Vec::new();
    archive::for_each_entry(&archive_path2, |entry| {
        entry_names2.push(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(
        entry_names, entry_names2,
        "Entry order should be deterministic across runs"
    );
}

/// Verifies that when creating an archive with multiple directory arguments, all entries from
/// the earlier directory argument appear before any entries from later directory arguments.
///
/// The test creates two directories (`dir_a` and `dir_b`) with files, runs `pna create` with
/// `dir_a` listed before `dir_b`, inspects the archive entries, and asserts that the highest
/// index of a `dir_a` entry is less than the lowest index of a `dir_b` entry.
///
/// # Examples
///
/// ```
/// // The test itself demonstrates usage; running the test ensures directory ordering is preserved:
/// create_preserves_multiple_directory_order();
/// ```
#[test]
fn create_preserves_multiple_directory_order() {
    setup();
    let dir = "create_preserves_multiple_directory_order";
    fs::create_dir_all(format!("{dir}/dir_a")).unwrap();
    fs::create_dir_all(format!("{dir}/dir_b")).unwrap();

    // dir_a has a large file (slow), dir_b has small files (fast)
    fs::write(format!("{dir}/dir_a/large.bin"), vec![0u8; 1024 * 1024]).unwrap();
    fs::write(format!("{dir}/dir_b/small1.txt"), b"small1").unwrap();
    fs::write(format!("{dir}/dir_b/small2.txt"), b"small2").unwrap();

    let archive_path = format!("{dir}/multi_dir.pna");

    // Create archive: dir_a first, then dir_b
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

    let mut entry_names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        entry_names.push(entry.header().path().to_string());
    })
    .unwrap();

    // Find indices of dir_a and dir_b entries
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

/// Precondition: Multiple files of varying sizes exist.
/// Action: Run `pna create --solid` with files in a specific order.
/// Expectation: Archive entries appear in the same order as CLI arguments.
#[test]
fn create_solid_preserves_cli_argument_order() {
    setup();
    let dir = "create_solid_preserves_cli_argument_order";
    fs::create_dir_all(dir).unwrap();

    let large_file = format!("{dir}/large.bin");
    let small_file = format!("{dir}/small.txt");
    let medium_file = format!("{dir}/medium.dat");

    fs::write(&large_file, vec![0u8; 1024 * 1024]).unwrap();
    fs::write(&small_file, b"small file").unwrap();
    fs::write(&medium_file, vec![1u8; 100 * 1024]).unwrap();

    let archive_path = format!("{dir}/ordered.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        "--solid",
        &small_file,
        &large_file,
        &medium_file,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        entry_names.push(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(entry_names.len(), 3);
    assert!(
        entry_names[0].ends_with("small.txt"),
        "First entry should be small.txt, got: {}",
        entry_names[0]
    );
    assert!(
        entry_names[1].ends_with("large.bin"),
        "Second entry should be large.bin, got: {}",
        entry_names[1]
    );
    assert!(
        entry_names[2].ends_with("medium.dat"),
        "Third entry should be medium.dat, got: {}",
        entry_names[2]
    );
}

/// Precondition: Multiple directories with files exist.
/// Action: Run `pna create --solid` with multiple directory arguments.
/// Expectation: Entries from first argument appear before second argument.
#[test]
fn create_solid_preserves_multiple_directory_order() {
    setup();
    let dir = "create_solid_preserves_multiple_directory_order";
    fs::create_dir_all(format!("{dir}/dir_a")).unwrap();
    fs::create_dir_all(format!("{dir}/dir_b")).unwrap();

    fs::write(format!("{dir}/dir_a/large.bin"), vec![0u8; 1024 * 1024]).unwrap();
    fs::write(format!("{dir}/dir_b/small1.txt"), b"small1").unwrap();
    fs::write(format!("{dir}/dir_b/small2.txt"), b"small2").unwrap();

    let archive_path = format!("{dir}/multi_dir_solid.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        "--solid",
        &format!("{dir}/dir_a"),
        &format!("{dir}/dir_b"),
        "--unstable",
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

/// Precondition: Multiple large files exist.
/// Action: Run `pna create --split` with files in a specific order.
/// Expectation: Concatenated archive has entries in argument order.
#[test]
fn create_split_preserves_entry_order() {
    setup();
    let dir = "create_split_preserves_entry_order";
    fs::create_dir_all(dir).unwrap();

    let large_file = format!("{dir}/large.bin");
    let small_file = format!("{dir}/small.txt");
    let medium_file = format!("{dir}/medium.dat");

    fs::write(&large_file, vec![0u8; 512 * 1024]).unwrap();
    fs::write(&small_file, b"small file").unwrap();
    fs::write(&medium_file, vec![1u8; 256 * 1024]).unwrap();

    let archive_path = format!("{dir}/split.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        "--split",
        "400KB",
        &small_file,
        &large_file,
        &medium_file,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Concatenate split archives
    let concat_path = format!("{dir}/concat.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "concat",
        &concat_path,
        "--overwrite",
        &archive_path,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_names = Vec::new();
    archive::for_each_entry(&concat_path, |entry| {
        entry_names.push(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(entry_names.len(), 3);
    assert!(
        entry_names[0].ends_with("small.txt"),
        "First entry should be small.txt, got: {}",
        entry_names[0]
    );
    assert!(
        entry_names[1].ends_with("large.bin"),
        "Second entry should be large.bin, got: {}",
        entry_names[1]
    );
    assert!(
        entry_names[2].ends_with("medium.dat"),
        "Third entry should be medium.dat, got: {}",
        entry_names[2]
    );
}