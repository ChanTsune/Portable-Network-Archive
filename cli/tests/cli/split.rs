use crate::utils::{EmbedExt, TestResources, diff::diff, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;
use std::io::Write;

#[test]
fn split_archive() {
    setup();
    TestResources::extract_in("raw/", "split_archive/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "split_archive/split.pna",
        "--overwrite",
        "split_archive/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "split_archive/split.pna",
        "--overwrite",
        "--max-size",
        "100kb",
        "--out-dir",
        "split_archive/split/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // check split file size
    for entry in fs::read_dir("split_archive/split/").unwrap() {
        assert!(fs::metadata(entry.unwrap().path()).unwrap().len() <= 100 * 1000);
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "split_archive/split/split.part1.pna",
        "--overwrite",
        "--out-dir",
        "split_archive/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // check completely extracted
    diff("split_archive/in/", "split_archive/out/").unwrap();
}

/// Test that split works when an entry's first chunk exceeds remaining space in current archive
/// but fits in a fresh archive with full capacity.
///
/// This tests the fix for the case where:
/// 1. Current archive is partially filled
/// 2. Next entry's first (unsplittable) chunk is larger than remaining space
/// 3. But the chunk fits in max_file_size
///
/// Before the fix, this would fail with "A chunk was detected that could not be divided..."
/// After the fix, it correctly creates a new archive part.
#[test]
fn split_archive_first_chunk_exceeds_remaining() {
    setup();

    // Create test directory with multiple files
    let test_dir = "split_first_chunk_test/in/";
    fs::create_dir_all(test_dir).unwrap();

    // Create several files with varying sizes to fill the archive strategically
    // File names affect FHED chunk size (header + name length)
    for i in 0..5 {
        let filename = format!("{}file{}.txt", test_dir, i);
        let mut file = fs::File::create(&filename).unwrap();
        // Write enough content to make splitting necessary
        file.write_all(&[b'A' + i; 20]).unwrap();
    }

    // Create archive from the test files
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "split_first_chunk_test/test.pna",
        "--overwrite",
        test_dir,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Split with a small size to trigger the edge case
    // MIN_SPLIT_PART_BYTES is 80 bytes, so use something just above that
    // This forces multiple splits with tight boundaries
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "split_first_chunk_test/test.pna",
        "--overwrite",
        "--max-size",
        "150",
        "--out-dir",
        "split_first_chunk_test/split/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify split parts were created and are within size limit
    let split_dir = fs::read_dir("split_first_chunk_test/split/").unwrap();
    let mut part_count = 0;
    for entry in split_dir {
        let path = entry.unwrap().path();
        let size = fs::metadata(&path).unwrap().len();
        assert!(
            size <= 150,
            "Split part {} exceeds max size: {} > 150",
            path.display(),
            size
        );
        part_count += 1;
    }
    assert!(
        part_count > 1,
        "Expected multiple split parts, got {}",
        part_count
    );

    // Extract and verify content matches original
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "split_first_chunk_test/split/test.part1.pna",
        "--overwrite",
        "--out-dir",
        "split_first_chunk_test/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify extracted content matches original
    diff(test_dir, "split_first_chunk_test/out/").unwrap();
}
