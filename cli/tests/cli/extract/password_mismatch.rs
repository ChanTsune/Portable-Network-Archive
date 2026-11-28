use crate::utils::setup;
use clap::Parser;
use pna::{Archive, FileEntryBuilder, WriteOptions};
use portable_network_archive::cli;
use std::{fs, io::Write, path::PathBuf};

/// Precondition: An AES-GCM encrypted archive entry and an existing file at the extraction target.
/// Action: Extract with `--overwrite` and a password that does not match the archive.
/// Expectation: Extraction fails and the existing file is left unchanged.
#[test]
fn extract_with_wrong_password_preserves_existing_file_aes_gcm() {
    setup();
    let archive_path = PathBuf::from("password_mismatch/archive.pna");
    let out_dir = PathBuf::from("password_mismatch/out");
    let target = out_dir.join("test.txt");
    let _ = fs::remove_dir_all("password_mismatch");
    fs::create_dir_all(archive_path.parent().unwrap()).unwrap();
    fs::create_dir_all(&out_dir).unwrap();

    let options = WriteOptions::builder()
        .encryption(pna::Encryption::AES)
        .cipher_mode(pna::CipherMode::GCM)
        .hash_algorithm(pna::HashAlgorithm::pbkdf2_sha256_with(Some(1000)))
        .password(Some("correct"))
        .build();
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    let mut builder = FileEntryBuilder::new_with_options("test.txt".into(), &options).unwrap();
    builder.write_all(b"new content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();
    archive.finalize().unwrap();

    fs::write(&target, b"original").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "password_mismatch/archive.pna",
        "--overwrite",
        "--out-dir",
        "password_mismatch/out",
        "--password",
        "wrong",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert_eq!(fs::read(&target).unwrap(), b"original");
}
