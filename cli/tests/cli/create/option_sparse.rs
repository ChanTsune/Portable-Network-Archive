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
