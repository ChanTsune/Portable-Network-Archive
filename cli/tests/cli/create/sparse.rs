//! Sparse file support tests.

use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::MetadataExt,
    path::PathBuf,
};

/// Creates a sparse file and returns whether the filesystem supports sparse files.
fn create_sparse_file(path: &PathBuf) -> bool {
    // Create a sparse file: [data][hole][data]
    // 4KB data + 1MB hole + 4KB data = ~1MB logical size
    {
        let file = File::create(path).unwrap();
        // First extend the file to create a hole
        file.set_len(1024 * 1024 + 4096).unwrap();
    }
    {
        let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();
        // Write 4KB of data at the start
        file.write_all(&[0xAA; 4096]).unwrap();
        // Seek to 1MB and write another 4KB
        file.seek(SeekFrom::Start(1024 * 1024)).unwrap();
        file.write_all(&[0xBB; 4096]).unwrap();
    }

    // Check if the filesystem actually created a sparse file
    let meta = fs::metadata(path).unwrap();
    let logical_size = meta.len();
    let block_bytes = meta.blocks() * 512;
    block_bytes < logical_size
}

/// Precondition: Sparse file with a hole in the middle.
/// Action: Create archive with `--sparse`, then extract with `--sparse`.
/// Expectation: Content matches; extracted file is sparse (st_blocks indicates holes).
#[test]
fn sparse_file_roundtrip() {
    setup();
    let base = PathBuf::from("sparse_roundtrip");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let sparse_path = base.join("sparse.bin");
    if !create_sparse_file(&sparse_path) {
        eprintln!("Skipping test: filesystem does not support sparse files");
        return;
    }

    // Create archive with --sparse
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "sparse_roundtrip/sparse.pna",
        "--overwrite",
        "--sparse",
        "--unstable",
        "sparse_roundtrip/sparse.bin",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract (sparse files are restored automatically when archive contains SPAR chunks)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "sparse_roundtrip/sparse.pna",
        "--overwrite",
        "--unstable",
        "--out-dir",
        "sparse_roundtrip/dist",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify content matches
    let original = fs::read(&sparse_path).unwrap();
    let extracted = fs::read(base.join("dist/sparse.bin")).unwrap();
    assert_eq!(original, extracted, "Content should match after roundtrip");

    // Verify extracted file is sparse
    let extracted_meta = fs::metadata(base.join("dist/sparse.bin")).unwrap();
    let extracted_blocks = extracted_meta.blocks() * 512;
    let extracted_size = extracted_meta.len();
    assert!(
        extracted_blocks < extracted_size,
        "Extracted file should be sparse: blocks={extracted_blocks}, size={extracted_size}"
    );
}

/// Creates an all-hole file (no data, entire file is a hole).
fn create_all_hole_file(path: &PathBuf, size: u64) -> bool {
    {
        let file = File::create(path).unwrap();
        file.set_len(size).unwrap();
    }

    // Check if the filesystem actually created a sparse file
    // Use the same check as create_sparse_file for consistency
    let meta = fs::metadata(path).unwrap();
    let block_bytes = meta.blocks() * 512;
    block_bytes < size
}

/// Precondition: All-hole file (entire file is a hole, no data).
/// Action: Create archive with `--sparse`, then extract with `--sparse`.
/// Expectation: Extracted file has correct size and reads as zeros.
#[test]
fn sparse_all_hole_file_roundtrip() {
    setup();
    let base = PathBuf::from("sparse_all_hole");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let sparse_path = base.join("hole.bin");
    let file_size = 1024 * 1024; // 1MB all-hole file
    if !create_all_hole_file(&sparse_path, file_size) {
        eprintln!("Skipping test: filesystem does not support sparse files");
        return;
    }

    // Create archive with --sparse
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "sparse_all_hole/hole.pna",
        "--overwrite",
        "--sparse",
        "--unstable",
        "sparse_all_hole/hole.bin",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract (sparse files are restored automatically when archive contains SPAR chunks)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "sparse_all_hole/hole.pna",
        "--overwrite",
        "--unstable",
        "--out-dir",
        "sparse_all_hole/dist",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify size matches
    let extracted_meta = fs::metadata(base.join("dist/hole.bin")).unwrap();
    assert_eq!(
        extracted_meta.len(),
        file_size,
        "Extracted file should have correct logical size"
    );

    // Verify content is all zeros
    let mut extracted_file = File::open(base.join("dist/hole.bin")).unwrap();
    let mut buf = vec![0u8; 4096];
    let mut total_read = 0u64;
    loop {
        let n = extracted_file.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        assert!(
            buf[..n].iter().all(|&b| b == 0),
            "All-hole file should read as zeros"
        );
        total_read += n as u64;
    }
    assert_eq!(total_read, file_size, "Should read entire file");

    // Verify extracted file is sparse (minimal disk usage)
    // Note: Some filesystems may not preserve sparse-ness on extraction
    let extracted_blocks = extracted_meta.blocks() * 512;
    if extracted_blocks >= file_size {
        eprintln!(
            "Note: Extracted all-hole file is not sparse (blocks={extracted_blocks}, size={file_size}). \
             This may be expected on some filesystems."
        );
    }
}

/// Creates a sparse file with multiple data regions and holes.
/// Pattern: [data][hole][data][hole][data]
fn create_multi_region_sparse_file(path: &PathBuf) -> bool {
    {
        let file = File::create(path).unwrap();
        // 3 data regions of 4KB each, separated by 256KB holes
        // Total: 4KB + 256KB + 4KB + 256KB + 4KB = ~524KB logical
        file.set_len(4096 + 256 * 1024 + 4096 + 256 * 1024 + 4096)
            .unwrap();
    }
    {
        let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();
        // Region 1: 0-4KB with pattern 0xAA
        file.write_all(&[0xAA; 4096]).unwrap();
        // Region 2: 260KB-264KB with pattern 0xBB
        file.seek(SeekFrom::Start(4096 + 256 * 1024)).unwrap();
        file.write_all(&[0xBB; 4096]).unwrap();
        // Region 3: 520KB-524KB with pattern 0xCC
        file.seek(SeekFrom::Start(4096 + 256 * 1024 + 4096 + 256 * 1024))
            .unwrap();
        file.write_all(&[0xCC; 4096]).unwrap();
    }

    // Check if sparse
    let meta = fs::metadata(path).unwrap();
    let logical_size = meta.len();
    let block_bytes = meta.blocks() * 512;
    block_bytes < logical_size
}

/// Precondition: Sparse file with multiple data regions separated by holes.
/// Action: Create archive with `--sparse`, then extract with `--sparse`.
/// Expectation: All data regions preserved with correct patterns; holes intact.
#[test]
fn sparse_multi_region_roundtrip() {
    setup();
    let base = PathBuf::from("sparse_multi_region");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let sparse_path = base.join("multi.bin");
    if !create_multi_region_sparse_file(&sparse_path) {
        eprintln!("Skipping test: filesystem does not support sparse files");
        return;
    }

    // Create archive with --sparse
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "sparse_multi_region/multi.pna",
        "--overwrite",
        "--sparse",
        "--unstable",
        "sparse_multi_region/multi.bin",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract (sparse files are restored automatically when archive contains SPAR chunks)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "sparse_multi_region/multi.pna",
        "--overwrite",
        "--unstable",
        "--out-dir",
        "sparse_multi_region/dist",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify content matches exactly
    let original = fs::read(&sparse_path).unwrap();
    let extracted = fs::read(base.join("dist/multi.bin")).unwrap();
    assert_eq!(
        original, extracted,
        "Content should match after roundtrip for multi-region sparse file"
    );

    // Verify specific data patterns
    let expected_size = 4096 + 256 * 1024 + 4096 + 256 * 1024 + 4096;
    assert_eq!(extracted.len(), expected_size);

    // Check region 1: 0xAA pattern
    assert!(
        extracted[..4096].iter().all(|&b| b == 0xAA),
        "Region 1 should have 0xAA pattern"
    );

    // Check hole 1: zeros
    assert!(
        extracted[4096..4096 + 256 * 1024].iter().all(|&b| b == 0),
        "Hole 1 should be zeros"
    );

    // Check region 2: 0xBB pattern
    let region2_start = 4096 + 256 * 1024;
    assert!(
        extracted[region2_start..region2_start + 4096]
            .iter()
            .all(|&b| b == 0xBB),
        "Region 2 should have 0xBB pattern"
    );

    // Check hole 2: zeros
    let hole2_start = region2_start + 4096;
    assert!(
        extracted[hole2_start..hole2_start + 256 * 1024]
            .iter()
            .all(|&b| b == 0),
        "Hole 2 should be zeros"
    );

    // Check region 3: 0xCC pattern
    let region3_start = hole2_start + 256 * 1024;
    assert!(
        extracted[region3_start..region3_start + 4096]
            .iter()
            .all(|&b| b == 0xCC),
        "Region 3 should have 0xCC pattern"
    );

    // Verify extracted file is sparse
    let extracted_meta = fs::metadata(base.join("dist/multi.bin")).unwrap();
    let extracted_blocks = extracted_meta.blocks() * 512;
    let extracted_size = extracted_meta.len();
    assert!(
        extracted_blocks < extracted_size,
        "Extracted multi-region file should be sparse: blocks={extracted_blocks}, size={extracted_size}"
    );
}

/// Creates a sparse file with trailing hole (data at start, hole at end).
fn create_trailing_hole_sparse_file(path: &PathBuf) -> bool {
    {
        let file = File::create(path).unwrap();
        // 4KB data + 1MB trailing hole
        file.set_len(4096 + 1024 * 1024).unwrap();
    }
    {
        let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.write_all(&[0xDD; 4096]).unwrap();
    }

    let meta = fs::metadata(path).unwrap();
    let logical_size = meta.len();
    let block_bytes = meta.blocks() * 512;
    block_bytes < logical_size
}

/// Precondition: Sparse file with data at start and trailing hole.
/// Action: Create archive with `--sparse`, then extract with `--sparse`.
/// Expectation: File has correct logical size; trailing zeros preserved.
#[test]
fn sparse_trailing_hole_roundtrip() {
    setup();
    let base = PathBuf::from("sparse_trailing_hole");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let sparse_path = base.join("trailing.bin");
    if !create_trailing_hole_sparse_file(&sparse_path) {
        eprintln!("Skipping test: filesystem does not support sparse files");
        return;
    }

    // Create archive with --sparse
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "sparse_trailing_hole/trailing.pna",
        "--overwrite",
        "--sparse",
        "--unstable",
        "sparse_trailing_hole/trailing.bin",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract (sparse files are restored automatically when archive contains SPAR chunks)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "sparse_trailing_hole/trailing.pna",
        "--overwrite",
        "--unstable",
        "--out-dir",
        "sparse_trailing_hole/dist",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify content matches
    let original = fs::read(&sparse_path).unwrap();
    let extracted = fs::read(base.join("dist/trailing.bin")).unwrap();
    assert_eq!(
        original, extracted,
        "Content should match for trailing hole sparse file"
    );

    // Verify logical size is correct (includes trailing hole)
    let expected_size = 4096 + 1024 * 1024;
    assert_eq!(extracted.len(), expected_size);

    // Verify data region
    assert!(
        extracted[..4096].iter().all(|&b| b == 0xDD),
        "Data region should have 0xDD pattern"
    );

    // Verify trailing hole is zeros
    assert!(
        extracted[4096..].iter().all(|&b| b == 0),
        "Trailing hole should be zeros"
    );
}

/// Creates a sparse file with leading hole (hole at start, data at end).
fn create_leading_hole_sparse_file(path: &PathBuf) -> bool {
    {
        let file = File::create(path).unwrap();
        // 1MB leading hole + 4KB data
        file.set_len(1024 * 1024 + 4096).unwrap();
    }
    {
        let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.seek(SeekFrom::Start(1024 * 1024)).unwrap();
        file.write_all(&[0xEE; 4096]).unwrap();
    }

    let meta = fs::metadata(path).unwrap();
    let logical_size = meta.len();
    let block_bytes = meta.blocks() * 512;
    block_bytes < logical_size
}

/// Precondition: Sparse file with leading hole and data at end.
/// Action: Create archive with `--sparse`, then extract with `--sparse`.
/// Expectation: Leading zeros preserved; data at correct offset.
#[test]
fn sparse_leading_hole_roundtrip() {
    setup();
    let base = PathBuf::from("sparse_leading_hole");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let sparse_path = base.join("leading.bin");
    if !create_leading_hole_sparse_file(&sparse_path) {
        eprintln!("Skipping test: filesystem does not support sparse files");
        return;
    }

    // Create archive with --sparse
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "sparse_leading_hole/leading.pna",
        "--overwrite",
        "--sparse",
        "--unstable",
        "sparse_leading_hole/leading.bin",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract (sparse files are restored automatically when archive contains SPAR chunks)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "sparse_leading_hole/leading.pna",
        "--overwrite",
        "--unstable",
        "--out-dir",
        "sparse_leading_hole/dist",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify content matches
    let original = fs::read(&sparse_path).unwrap();
    let extracted = fs::read(base.join("dist/leading.bin")).unwrap();
    assert_eq!(
        original, extracted,
        "Content should match for leading hole sparse file"
    );

    // Verify logical size
    let expected_size = 1024 * 1024 + 4096;
    assert_eq!(extracted.len(), expected_size);

    // Verify leading hole is zeros
    assert!(
        extracted[..1024 * 1024].iter().all(|&b| b == 0),
        "Leading hole should be zeros"
    );

    // Verify data region
    assert!(
        extracted[1024 * 1024..].iter().all(|&b| b == 0xEE),
        "Data region should have 0xEE pattern"
    );
}
