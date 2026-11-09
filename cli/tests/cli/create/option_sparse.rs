use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

fn assert_dense_file_round_trips(name: &str, content: &[u8]) {
    let root = format!("create_sparse_dense_{name}");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(format!("{root}/in")).unwrap();
    fs::write(format!("{root}/in/file.bin"), content).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "-f",
        &format!("{root}/archive.pna"),
        "--overwrite",
        "--unstable",
        "--sparse",
        &format!("{root}/in/file.bin"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "extract",
        "-f",
        &format!("{root}/archive.pna"),
        "--overwrite",
        "--out-dir",
        &format!("{root}/out"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let extracted = fs::read(format!("{root}/out/{root}/in/file.bin")).unwrap();
    assert_eq!(extracted.len(), content.len());
    assert!(extracted == content);
}

/// Precondition: A fully allocated file smaller than the in-memory read threshold.
/// Action: Run `pna create --sparse` and extract the result.
/// Expectation: The extracted bytes equal the source.
#[test]
fn sparse_option_keeps_small_dense_file_content() {
    setup();
    assert_dense_file_round_trips("small", b"dense file content");
}

/// Precondition: A fully allocated file at or above the in-memory read threshold.
/// Action: Run `pna create --sparse` and extract the result.
/// Expectation: The extracted bytes equal the source.
#[test]
fn sparse_option_keeps_large_dense_file_content() {
    setup();
    let content: Vec<u8> = (0..(50 * 1024 * 1024 + 1))
        .map(|i| (i % 251) as u8)
        .collect();
    assert_dense_file_round_trips("large", &content);
}

#[cfg(unix)]
fn file_is_sparse(path: &str) -> bool {
    use std::os::unix::fs::MetadataExt;
    let meta = fs::metadata(path).unwrap();
    meta.blocks() * 512 < meta.len()
}

#[cfg(unix)]
fn assert_sparse_file_round_trips(name: &str, size: u64, extents: &[(u64, u8, usize)]) {
    use std::{
        fs::File,
        io::{Seek, SeekFrom, Write},
    };

    let root = format!("create_sparse_{name}");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(format!("{root}/in")).unwrap();
    let source = format!("{root}/in/file.bin");
    {
        let file = File::create(&source).unwrap();
        file.set_len(size).unwrap();
    }
    let mut expected = vec![0u8; size as usize];
    {
        let mut file = fs::OpenOptions::new().write(true).open(&source).unwrap();
        for &(offset, byte, len) in extents {
            file.seek(SeekFrom::Start(offset)).unwrap();
            file.write_all(&vec![byte; len]).unwrap();
            expected[offset as usize..offset as usize + len].fill(byte);
        }
    }

    skip_unless!("sparse", file_is_sparse(&source));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "-f",
        &format!("{root}/archive.pna"),
        "--overwrite",
        "--unstable",
        "--sparse",
        &source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "extract",
        "-f",
        &format!("{root}/archive.pna"),
        "--overwrite",
        "--out-dir",
        &format!("{root}/out"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let extracted_path = format!("{root}/out/{source}");
    let extracted = fs::read(&extracted_path).unwrap();
    assert_eq!(extracted.len(), expected.len());
    let mismatch = extracted
        .iter()
        .zip(expected.iter())
        .position(|(actual, expected)| actual != expected);
    assert!(mismatch.is_none(), "first differing byte: {mismatch:?}");
    assert!(
        file_is_sparse(&extracted_path),
        "extracted file should be sparse"
    );
}

/// Precondition: A sparse file with a hole between two data regions.
/// Action: Run `pna create --sparse` and extract the result.
/// Expectation: The extracted bytes equal the source layout and the extracted file is sparse.
#[test]
#[cfg(unix)]
fn sparse_option_restores_file_with_middle_hole() {
    setup();
    assert_sparse_file_round_trips(
        "middle_hole",
        1024 * 1024 + 4096,
        &[(0, 0xAA, 4096), (1024 * 1024, 0xBB, 4096)],
    );
}

/// Precondition: A sparse file that is entirely a hole.
/// Action: Run `pna create --sparse` and extract the result.
/// Expectation: The extracted file has the source size, reads as zeros, and is sparse.
#[test]
#[cfg(unix)]
fn sparse_option_restores_all_hole_file() {
    setup();
    assert_sparse_file_round_trips("all_hole", 1024 * 1024, &[]);
}

/// Precondition: A sparse file with multiple data regions separated by holes.
/// Action: Run `pna create --sparse` and extract the result.
/// Expectation: The extracted bytes equal the source layout and the extracted file is sparse.
#[test]
#[cfg(unix)]
fn sparse_option_restores_multi_region_file() {
    setup();
    assert_sparse_file_round_trips(
        "multi_region",
        4096 + 256 * 1024 + 4096 + 256 * 1024 + 4096,
        &[
            (0, 0xAA, 4096),
            (4096 + 256 * 1024, 0xBB, 4096),
            (4096 + 256 * 1024 + 4096 + 256 * 1024, 0xCC, 4096),
        ],
    );
}

/// Precondition: A sparse file with data at the start and a trailing hole.
/// Action: Run `pna create --sparse` and extract the result.
/// Expectation: The extracted bytes equal the source layout and the extracted file is sparse.
#[test]
#[cfg(unix)]
fn sparse_option_restores_trailing_hole_file() {
    setup();
    assert_sparse_file_round_trips("trailing_hole", 4096 + 1024 * 1024, &[(0, 0xDD, 4096)]);
}

/// Precondition: A sparse file with a leading hole and data at the end.
/// Action: Run `pna create --sparse` and extract the result.
/// Expectation: The extracted bytes equal the source layout and the extracted file is sparse.
#[test]
#[cfg(unix)]
fn sparse_option_restores_leading_hole_file() {
    setup();
    assert_sparse_file_round_trips(
        "leading_hole",
        1024 * 1024 + 4096,
        &[(1024 * 1024, 0xEE, 4096)],
    );
}
