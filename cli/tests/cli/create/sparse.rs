//! Sparse file support tests.

use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::{
    fs::{self, File},
    io::{Seek, SeekFrom, Write},
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

    // Extract with --sparse
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "sparse_roundtrip/sparse.pna",
        "--overwrite",
        "--sparse",
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
