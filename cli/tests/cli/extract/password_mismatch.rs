use crate::utils::setup;
use clap::Parser;
use pna::{Archive, FileEntryBuilder, WriteOptions};
use portable_network_archive::cli;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

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

/// Precondition: An AES-GCM encrypted archive entry, and a symlink at the extraction target pointing to an existing file.
/// Action: Extract with `--overwrite` and a password that does not match the archive.
/// Expectation: Extraction fails and the symlink and its target file are left unchanged.
#[test]
fn extract_with_wrong_password_preserves_existing_symlink_aes_gcm() {
    setup();
    let archive_path = PathBuf::from("password_mismatch_symlink/archive.pna");
    let out_dir = PathBuf::from("password_mismatch_symlink/out");
    let target = out_dir.join("test.txt");
    let real = out_dir.join("real.txt");
    let _ = fs::remove_dir_all("password_mismatch_symlink");
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

    fs::write(&real, b"original").unwrap();
    pna::fs::symlink(Path::new("real.txt"), &target).unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "password_mismatch_symlink/archive.pna",
        "--overwrite",
        "--out-dir",
        "password_mismatch_symlink/out",
        "--password",
        "wrong",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert!(
        fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_link(&target).unwrap(), Path::new("real.txt"));
    assert_eq!(fs::read(&real).unwrap(), b"original");
}

/// Precondition: An AES-GCM encrypted archive entry below a directory component, and an existing regular file occupying that component in the output directory.
/// Action: Extract with `--overwrite` and a password that does not match the archive.
/// Expectation: Extraction fails and the existing regular file is left unchanged.
#[test]
fn extract_with_wrong_password_preserves_blocking_parent_file_aes_gcm() {
    setup();
    let archive_path = PathBuf::from("password_mismatch_parent/archive.pna");
    let out_dir = PathBuf::from("password_mismatch_parent/out");
    let blocking = out_dir.join("blocked");
    let _ = fs::remove_dir_all("password_mismatch_parent");
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
    let mut builder =
        FileEntryBuilder::new_with_options("blocked/test.txt".into(), &options).unwrap();
    builder.write_all(b"new content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();
    archive.finalize().unwrap();

    fs::write(&blocking, b"original").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "password_mismatch_parent/archive.pna",
        "--overwrite",
        "--out-dir",
        "password_mismatch_parent/out",
        "--password",
        "wrong",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert!(fs::metadata(&blocking).unwrap().is_file());
    assert_eq!(fs::read(&blocking).unwrap(), b"original");
}
