#![cfg(not(target_family = "wasm"))]

use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, ReadOptions};
use predicates::prelude::predicate;
use std::fs;
use std::io::{Cursor, Read};

fn read_archive_entries(bytes: &[u8]) -> Vec<(String, String)> {
    let mut archive = Archive::read_header(Cursor::new(bytes)).unwrap();
    archive
        .entries()
        .extract_solid_entries(&ReadOptions::builder().build())
        .map(|entry| {
            let entry = entry.unwrap();
            let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
            let mut content = String::new();
            reader.read_to_string(&mut content).unwrap();
            (entry.name().to_string(), content)
        })
        .collect()
}

/// Precondition: A file exists in the filesystem.
/// Action: Create archive using bsdtar with --mac-metadata --unstable flags.
/// Expectation: Command writes an archive containing the requested file data.
#[test]
fn bsdtar_create_with_mac_metadata_writes_archive_entry() {
    setup();
    let file = "bsdtar_mac_metadata_option_accepted.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let output = cmd
        .arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("--unstable")
        .arg("--mac-metadata")
        .arg(file)
        .assert()
        .success();

    assert_eq!(
        read_archive_entries(output.get_output().stdout.as_slice()),
        vec![(file.to_string(), "test content".to_string())],
        "--mac-metadata create should emit an archive with the requested entry"
    );
}

/// Precondition: A file exists in the filesystem.
/// Action: Create archive using bsdtar with --no-mac-metadata --unstable flags.
/// Expectation: Command succeeds (option is recognized and accepted).
#[test]
fn bsdtar_no_mac_metadata_option_accepted() {
    setup();
    let file = "bsdtar_no_mac_metadata_option_accepted.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("--unstable")
        .arg("--no-mac-metadata")
        .arg(file)
        .assert()
        .success();
}

/// Precondition: A file exists in the filesystem.
/// Action: Attempt to use --mac-metadata without --unstable flag.
/// Expectation: Command fails because --mac-metadata requires --unstable.
#[test]
fn bsdtar_mac_metadata_requires_unstable() {
    setup();
    let file = "bsdtar_mac_metadata_requires_unstable.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("--mac-metadata")
        .arg(file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("--unstable"));
}

/// Precondition: A file exists in the filesystem.
/// Action: Attempt to use both --mac-metadata and --no-mac-metadata together.
/// Expectation: Command fails because the options are mutually exclusive.
#[test]
fn bsdtar_mac_metadata_and_no_mac_metadata_mutually_exclusive() {
    setup();
    let file = "bsdtar_mac_metadata_mutually_exclusive.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("--unstable")
        .arg("--mac-metadata")
        .arg("--no-mac-metadata")
        .arg(file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Precondition: A file exists in the filesystem.
/// Action: Extract archive using bsdtar with --mac-metadata --unstable flags.
/// Expectation: Command succeeds (option is recognized for extract mode).
#[test]
fn bsdtar_extract_mac_metadata_option_accepted() {
    setup();
    fs::create_dir_all("bsdtar_extract_mac_metadata_dir").unwrap();
    fs::write("bsdtar_extract_mac_metadata_dir/test.txt", "test content").unwrap();
    fs::create_dir_all("bsdtar_extract_mac_metadata_dir/out").unwrap();

    // Create an archive first
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            "bsdtar_extract_mac_metadata_dir/test.pna",
            "--overwrite",
            "bsdtar_extract_mac_metadata_dir/test.txt",
        ])
        .assert()
        .success();

    // Extract with --mac-metadata
    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "-x",
            "--unstable",
            "--mac-metadata",
            "-f",
            "bsdtar_extract_mac_metadata_dir/test.pna",
            "--out-dir",
            "bsdtar_extract_mac_metadata_dir/out",
        ])
        .assert()
        .success();
}

/// Precondition: A file exists in the filesystem.
/// Action: Use bsdtar append mode with --mac-metadata --unstable flags.
/// Expectation: Command succeeds (option is recognized for append mode).
#[test]
fn bsdtar_append_mac_metadata_option_accepted() {
    setup();
    fs::create_dir_all("bsdtar_append_mac_metadata_dir").unwrap();
    fs::write("bsdtar_append_mac_metadata_dir/file1.txt", "test content 1").unwrap();
    fs::write("bsdtar_append_mac_metadata_dir/file2.txt", "test content 2").unwrap();

    // Create an archive first
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            "bsdtar_append_mac_metadata_dir/test.pna",
            "--overwrite",
            "bsdtar_append_mac_metadata_dir/file1.txt",
        ])
        .assert()
        .success();

    // Append with --mac-metadata
    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "-r",
            "--unstable",
            "--mac-metadata",
            "-f",
            "bsdtar_append_mac_metadata_dir/test.pna",
            "bsdtar_append_mac_metadata_dir/file2.txt",
        ])
        .assert()
        .success();
}

/// Precondition: A file exists in the filesystem.
/// Action: Use bsdtar update mode with --mac-metadata --unstable flags.
/// Expectation: Command succeeds (option is recognized for update mode).
#[test]
fn bsdtar_update_mac_metadata_option_accepted() {
    setup();
    fs::create_dir_all("bsdtar_update_mac_metadata_dir").unwrap();
    fs::write("bsdtar_update_mac_metadata_dir/test.txt", "test content").unwrap();

    // Create an archive first
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            "bsdtar_update_mac_metadata_dir/test.pna",
            "--overwrite",
            "bsdtar_update_mac_metadata_dir/test.txt",
        ])
        .assert()
        .success();

    // Update the file
    fs::write("bsdtar_update_mac_metadata_dir/test.txt", "updated content").unwrap();

    // Update with --mac-metadata
    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "-u",
            "--unstable",
            "--mac-metadata",
            "-f",
            "bsdtar_update_mac_metadata_dir/test.pna",
            "bsdtar_update_mac_metadata_dir/test.txt",
        ])
        .assert()
        .success();
}

// macOS-specific tests that verify xattr preservation
#[cfg(target_os = "macos")]
mod macos_tests {
    use super::*;
    use std::process::Command;

    /// Precondition: A file with extended attributes exists on macOS.
    /// Action: Create archive with --mac-metadata and extract it.
    /// Expectation: Extended attributes are preserved in the archive and restored on extraction.
    #[test]
    fn bsdtar_mac_metadata_preserves_xattrs() {
        setup();
        fs::create_dir_all("bsdtar_mac_metadata_xattr_dir").unwrap();
        fs::write("bsdtar_mac_metadata_xattr_dir/test.txt", "test content").unwrap();
        fs::create_dir_all("bsdtar_mac_metadata_xattr_dir/out").unwrap();

        // Set xattr
        let status = Command::new("xattr")
            .args([
                "-w",
                "com.example.test",
                "test_value",
                "bsdtar_mac_metadata_xattr_dir/test.txt",
            ])
            .status()
            .expect("Failed to run xattr");
        assert!(
            status.success(),
            "xattr failed with code: {:?}",
            status.code()
        );

        // Create archive with --mac-metadata
        cargo_bin_cmd!("pna")
            .args([
                "compat",
                "bsdtar",
                "-c",
                "--unstable",
                "--mac-metadata",
                "--overwrite",
                "-f",
                "bsdtar_mac_metadata_xattr_dir/test.pna",
                "bsdtar_mac_metadata_xattr_dir/test.txt",
            ])
            .assert()
            .success();

        // Extract with --mac-metadata
        cargo_bin_cmd!("pna")
            .args([
                "compat",
                "bsdtar",
                "-x",
                "--unstable",
                "--mac-metadata",
                "-f",
                "bsdtar_mac_metadata_xattr_dir/test.pna",
                "--out-dir",
                "bsdtar_mac_metadata_xattr_dir/out",
                "--overwrite",
            ])
            .assert()
            .success();

        // Verify xattr is preserved
        let output = Command::new("xattr")
            .args([
                "-p",
                "com.example.test",
                "bsdtar_mac_metadata_xattr_dir/out/bsdtar_mac_metadata_xattr_dir/test.txt",
            ])
            .output()
            .expect("Failed to read xattr");

        assert!(output.status.success());
        let extracted_value = String::from_utf8_lossy(&output.stdout);
        assert_eq!(extracted_value.trim(), "test_value");
    }

    /// Precondition: A file with extended attributes exists on macOS.
    /// Action: Create archive with --no-mac-metadata and extract it.
    /// Expectation: Extended attributes are NOT preserved.
    #[test]
    fn bsdtar_no_mac_metadata_excludes_xattrs() {
        setup();
        fs::create_dir_all("bsdtar_no_mac_metadata_xattr_dir").unwrap();
        fs::write("bsdtar_no_mac_metadata_xattr_dir/test.txt", "test content").unwrap();
        fs::create_dir_all("bsdtar_no_mac_metadata_xattr_dir/out").unwrap();

        // Set xattr
        let status = Command::new("xattr")
            .args([
                "-w",
                "com.example.test",
                "test_value",
                "bsdtar_no_mac_metadata_xattr_dir/test.txt",
            ])
            .status()
            .expect("Failed to run xattr");
        assert!(
            status.success(),
            "xattr failed with code: {:?}",
            status.code()
        );

        // Create archive with --no-mac-metadata
        cargo_bin_cmd!("pna")
            .args([
                "compat",
                "bsdtar",
                "-c",
                "--unstable",
                "--no-mac-metadata",
                "--overwrite",
                "-f",
                "bsdtar_no_mac_metadata_xattr_dir/test.pna",
                "bsdtar_no_mac_metadata_xattr_dir/test.txt",
            ])
            .assert()
            .success();

        // Extract with --mac-metadata (even if we try to restore, nothing should be there)
        cargo_bin_cmd!("pna")
            .args([
                "compat",
                "bsdtar",
                "-x",
                "--unstable",
                "--mac-metadata",
                "-f",
                "bsdtar_no_mac_metadata_xattr_dir/test.pna",
                "--out-dir",
                "bsdtar_no_mac_metadata_xattr_dir/out",
                "--overwrite",
            ])
            .assert()
            .success();

        // Verify xattr is NOT preserved
        let output = Command::new("xattr")
            .args([
                "-p",
                "com.example.test",
                "bsdtar_no_mac_metadata_xattr_dir/out/bsdtar_no_mac_metadata_xattr_dir/test.txt",
            ])
            .output()
            .expect("Failed to check xattr");

        // xattr command should fail because the attribute doesn't exist
        assert!(!output.status.success());
    }
}
