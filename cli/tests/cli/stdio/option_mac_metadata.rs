#![cfg(not(target_family = "wasm"))]

use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::predicate;
use std::fs;

/// Precondition: A file exists in the filesystem.
/// Action: Create archive using stdio with --mac-metadata --unstable flags.
/// Expectation: Command succeeds (option is recognized and accepted).
#[test]
fn stdio_mac_metadata_option_accepted() {
    setup();
    let file = "stdio_mac_metadata_option_accepted.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--unstable")
        .arg("--mac-metadata")
        .arg(file)
        .assert()
        .success();
}

/// Precondition: A file exists in the filesystem.
/// Action: Create archive using stdio with --no-mac-metadata --unstable flags.
/// Expectation: Command succeeds (option is recognized and accepted).
#[test]
fn stdio_no_mac_metadata_option_accepted() {
    setup();
    let file = "stdio_no_mac_metadata_option_accepted.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
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
fn stdio_mac_metadata_requires_unstable() {
    setup();
    let file = "stdio_mac_metadata_requires_unstable.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
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
fn stdio_mac_metadata_and_no_mac_metadata_mutually_exclusive() {
    setup();
    let file = "stdio_mac_metadata_mutually_exclusive.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
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
/// Action: Extract archive using stdio with --mac-metadata --unstable flags.
/// Expectation: Command succeeds (option is recognized for extract mode).
#[test]
fn stdio_extract_mac_metadata_option_accepted() {
    setup();
    fs::create_dir_all("stdio_extract_mac_metadata_dir").unwrap();
    fs::write("stdio_extract_mac_metadata_dir/test.txt", "test content").unwrap();
    fs::create_dir_all("stdio_extract_mac_metadata_dir/out").unwrap();

    // Create an archive first
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "stdio_extract_mac_metadata_dir/test.pna",
            "--overwrite",
            "stdio_extract_mac_metadata_dir/test.txt",
        ])
        .assert()
        .success();

    // Extract with --mac-metadata
    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "stdio",
            "-x",
            "--unstable",
            "--mac-metadata",
            "-f",
            "stdio_extract_mac_metadata_dir/test.pna",
            "--out-dir",
            "stdio_extract_mac_metadata_dir/out",
        ])
        .assert()
        .success();
}

/// Precondition: A file exists in the filesystem.
/// Action: Use stdio append mode with --mac-metadata --unstable flags.
/// Expectation: Command succeeds (option is recognized for append mode).
#[test]
fn stdio_append_mac_metadata_option_accepted() {
    setup();
    fs::create_dir_all("stdio_append_mac_metadata_dir").unwrap();
    fs::write("stdio_append_mac_metadata_dir/file1.txt", "test content 1").unwrap();
    fs::write("stdio_append_mac_metadata_dir/file2.txt", "test content 2").unwrap();

    // Create an archive first
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "stdio_append_mac_metadata_dir/test.pna",
            "--overwrite",
            "stdio_append_mac_metadata_dir/file1.txt",
        ])
        .assert()
        .success();

    // Append with --mac-metadata
    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "stdio",
            "-r",
            "--unstable",
            "--mac-metadata",
            "-f",
            "stdio_append_mac_metadata_dir/test.pna",
            "stdio_append_mac_metadata_dir/file2.txt",
        ])
        .assert()
        .success();
}

/// Precondition: A file exists in the filesystem.
/// Action: Use stdio update mode with --mac-metadata --unstable flags.
/// Expectation: Command succeeds (option is recognized for update mode).
#[test]
fn stdio_update_mac_metadata_option_accepted() {
    setup();
    fs::create_dir_all("stdio_update_mac_metadata_dir").unwrap();
    fs::write("stdio_update_mac_metadata_dir/test.txt", "test content").unwrap();

    // Create an archive first
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "stdio_update_mac_metadata_dir/test.pna",
            "--overwrite",
            "stdio_update_mac_metadata_dir/test.txt",
        ])
        .assert()
        .success();

    // Update the file
    fs::write("stdio_update_mac_metadata_dir/test.txt", "updated content").unwrap();

    // Update with --mac-metadata
    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "stdio",
            "-u",
            "--unstable",
            "--mac-metadata",
            "-f",
            "stdio_update_mac_metadata_dir/test.pna",
            "stdio_update_mac_metadata_dir/test.txt",
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
    fn stdio_mac_metadata_preserves_xattrs() {
        setup();
        fs::create_dir_all("stdio_mac_metadata_xattr_dir").unwrap();
        fs::write("stdio_mac_metadata_xattr_dir/test.txt", "test content").unwrap();
        fs::create_dir_all("stdio_mac_metadata_xattr_dir/out").unwrap();

        // Set xattr
        let status = Command::new("xattr")
            .args([
                "-w",
                "com.example.test",
                "test_value",
                "stdio_mac_metadata_xattr_dir/test.txt",
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
                "experimental",
                "stdio",
                "-c",
                "--unstable",
                "--mac-metadata",
                "--overwrite",
                "-f",
                "stdio_mac_metadata_xattr_dir/test.pna",
                "stdio_mac_metadata_xattr_dir/test.txt",
            ])
            .assert()
            .success();

        // Extract with --mac-metadata
        cargo_bin_cmd!("pna")
            .args([
                "experimental",
                "stdio",
                "-x",
                "--unstable",
                "--mac-metadata",
                "-f",
                "stdio_mac_metadata_xattr_dir/test.pna",
                "--out-dir",
                "stdio_mac_metadata_xattr_dir/out",
                "--overwrite",
            ])
            .assert()
            .success();

        // Verify xattr is preserved
        let output = Command::new("xattr")
            .args([
                "-p",
                "com.example.test",
                "stdio_mac_metadata_xattr_dir/out/stdio_mac_metadata_xattr_dir/test.txt",
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
    fn stdio_no_mac_metadata_excludes_xattrs() {
        setup();
        fs::create_dir_all("stdio_no_mac_metadata_xattr_dir").unwrap();
        fs::write("stdio_no_mac_metadata_xattr_dir/test.txt", "test content").unwrap();
        fs::create_dir_all("stdio_no_mac_metadata_xattr_dir/out").unwrap();

        // Set xattr
        let status = Command::new("xattr")
            .args([
                "-w",
                "com.example.test",
                "test_value",
                "stdio_no_mac_metadata_xattr_dir/test.txt",
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
                "experimental",
                "stdio",
                "-c",
                "--unstable",
                "--no-mac-metadata",
                "--overwrite",
                "-f",
                "stdio_no_mac_metadata_xattr_dir/test.pna",
                "stdio_no_mac_metadata_xattr_dir/test.txt",
            ])
            .assert()
            .success();

        // Extract with --mac-metadata (even if we try to restore, nothing should be there)
        cargo_bin_cmd!("pna")
            .args([
                "experimental",
                "stdio",
                "-x",
                "--unstable",
                "--mac-metadata",
                "-f",
                "stdio_no_mac_metadata_xattr_dir/test.pna",
                "--out-dir",
                "stdio_no_mac_metadata_xattr_dir/out",
                "--overwrite",
            ])
            .assert()
            .success();

        // Verify xattr is NOT preserved
        let output = Command::new("xattr")
            .args([
                "-p",
                "com.example.test",
                "stdio_no_mac_metadata_xattr_dir/out/stdio_no_mac_metadata_xattr_dir/test.txt",
            ])
            .output()
            .expect("Failed to check xattr");

        // xattr command should fail because the attribute doesn't exist
        assert!(!output.status.success());
    }
}
