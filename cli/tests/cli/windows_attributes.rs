#![cfg(windows)]

use std::fs::{self, File};
use std::io::{self, Write};
use std::os::windows::fs::MetadataExt; // For checking ReadOnly via metadata
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::prelude::*;
use tempfile::TempDir;

// Assuming utils are in the same test crate or accessible
// For PnaCommandExt etc.
mod utils;
use utils::{CommandExt, OutputExt, PnaCommandExt};

const FILE_ATTRIBUTE_READONLY: u32 = 0x00000001;
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;
// System attribute, often set with Hidden for OS files
// const FILE_ATTRIBUTE_SYSTEM: u32 = 0x00000004;

// Helper function to set file attributes using the `attrib` command
fn set_windows_file_attributes(path: &Path, make_readonly: bool, make_hidden: bool) -> io::Result<()> {
    let mut command = StdCommand::new("cmd");
    command.arg("/C").arg("attrib");
    if make_readonly {
        command.arg("+R");
    } else {
        command.arg("-R");
    }
    if make_hidden {
        command.arg("+H");
    } else {
        command.arg("-H");
    }
    command.arg(path.as_os_str());

    let status = command.status()?;
    if status.success() {
        // Wait a bit to ensure attributes are applied before proceeding
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("attrib command failed with status: {}", status),
        ))
    }
}

// Helper function to get file attributes using the `attrib` command (for verification)
// This is a bit more complex to parse robustly, so we'll mostly rely on behavior verification
// or pna xattr get for the stored value.
// For direct attribute checking, Rust's std::fs::Metadata is limited (only readonly).
// A full get_windows_file_attributes would ideally use GetFileAttributesW from windows-sys
// but that's not directly callable from an external test crate without FFI or IPC.
// We will use this primarily to check if a file IS readonly or IS hidden.
fn check_windows_file_attributes(path: &Path, should_be_readonly: bool, should_be_hidden: bool) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    let is_readonly = metadata.permissions().readonly();

    if should_be_readonly != is_readonly {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Readonly attribute mismatch for {:?}: expected {}, got {}",
                path, should_be_readonly, is_readonly
            ),
        ));
    }

    // Checking "hidden" is trickier with std::fs::metadata.
    // On Windows, file_attributes() is available on MetadataExt.
    let attributes = metadata.file_attributes();
    let is_hidden = (attributes & FILE_ATTRIBUTE_HIDDEN) != 0;

    if should_be_hidden != is_hidden {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Hidden attribute mismatch for {:?}: expected {}, got {} (all attributes: {:x})",
                path, should_be_hidden, is_hidden, attributes
            ),
        ));
    }
    Ok(())
}

// Helper to create a dummy file
fn create_dummy_file(dir: &TempDir, name: &str, content: &[u8]) -> io::Result<PathBuf> {
    let file_path = dir.path().join(name);
    let mut file = File::create(&file_path)?;
    file.write_all(content)?;
    Ok(file_path)
}

#[test]
fn test_store_windows_readonly_attribute() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = create_dummy_file(&temp_dir, "readonly.txt", b"test content")?;

    set_windows_file_attributes(&test_file, true, false)?;
    
    let archive_path = temp_dir.path().join("archive.pna");

    let mut cmd = StdCommand::pna();
    cmd.arg("create")
        .arg(&archive_path)
        .arg(test_file.file_name().unwrap())
        .arg("--store-windows-attributes")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    let mut cmd_xattr = StdCommand::pna();
    cmd_xattr
        .arg("xattr")
        .arg("get")
        .arg(&archive_path)
        .arg(test_file.file_name().unwrap())
        .arg("--name")
        .arg("windows.file_attributes")
        .arg("--encoding")
        .arg("hex")
        .current_dir(temp_dir.path());
    
    let output = cmd_xattr.output()?;
    cmd_xattr.assert().success();
    let stdout = String::from_utf8(output.stdout)?;
    
    // Expected output might be "windows.file_attributes=0x1" or similar,
    // sometimes it might include archive path or file path.
    // We expect the stored attributes to be at least 0x1. Other bits like Archive (0x20) might be set by OS.
    // So we check if the readonly bit is set.
    let extracted_hex = stdout.split('=').last().unwrap_or("").trim();
    let attributes_val = u32::from_str_radix(extracted_hex.strip_prefix("0x").unwrap_or(extracted_hex), 16)?;
    assert_ne!(attributes_val & FILE_ATTRIBUTE_READONLY, 0, "ReadOnly attribute (0x1) was not stored. Output: {}", stdout);

    Ok(())
}

#[test]
fn test_store_and_restore_windows_readonly_attribute() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file_name = "readonly_restore.txt";
    let test_file = create_dummy_file(&temp_dir, test_file_name, b"test content")?;

    set_windows_file_attributes(&test_file, true, false)?;

    let archive_path = temp_dir.path().join("archive_readonly_restore.pna");
    StdCommand::pna()
        .arg("create")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--store-windows-attributes")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let output_dir = temp_dir.path().join("output_readonly");
    fs::create_dir(&output_dir)?;

    StdCommand::pna()
        .arg("extract")
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&output_dir)
        .arg("--restore-windows-attributes")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let extracted_file_path = output_dir.join(test_file_name);
    assert!(extracted_file_path.exists());
    check_windows_file_attributes(&extracted_file_path, true, false)?;

    Ok(())
}

#[test]
fn test_restore_without_flag_does_not_apply_readonly() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file_name = "readonly_no_restore.txt";
    let test_file = create_dummy_file(&temp_dir, test_file_name, b"test content")?;

    set_windows_file_attributes(&test_file, true, false)?;

    let archive_path = temp_dir.path().join("archive_readonly_no_restore.pna");
    StdCommand::pna()
        .arg("create")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--store-windows-attributes")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let output_dir_no_restore = temp_dir.path().join("output_no_restore");
    fs::create_dir(&output_dir_no_restore)?;

    StdCommand::pna()
        .arg("extract")
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&output_dir_no_restore)
        // No --restore-windows-attributes flag
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let extracted_file_path = output_dir_no_restore.join(test_file_name);
    assert!(extracted_file_path.exists());
    // Should NOT be readonly, should be hidden false (default)
    check_windows_file_attributes(&extracted_file_path, false, false)?;

    Ok(())
}

#[test]
fn test_store_and_restore_multiple_attributes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file_name = "multi_attr.txt";
    let test_file = create_dummy_file(&temp_dir, test_file_name, b"multi content")?;

    // Set ReadOnly and Hidden
    set_windows_file_attributes(&test_file, true, true)?;

    let archive_path = temp_dir.path().join("archive_multi.pna");
    StdCommand::pna()
        .arg("create")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--store-windows-attributes")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Verify stored xattr
    let mut cmd_xattr = StdCommand::pna();
    cmd_xattr
        .arg("xattr")
        .arg("get")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--name")
        .arg("windows.file_attributes")
        .arg("--encoding")
        .arg("hex")
        .current_dir(temp_dir.path());
    
    let output = cmd_xattr.output()?;
    cmd_xattr.assert().success();
    let stdout = String::from_utf8(output.stdout)?;
    let extracted_hex = stdout.split('=').last().unwrap_or("").trim();
    let attributes_val = u32::from_str_radix(extracted_hex.strip_prefix("0x").unwrap_or(extracted_hex), 16)?;
    
    assert_ne!(attributes_val & FILE_ATTRIBUTE_READONLY, 0, "ReadOnly attribute (0x1) was not stored. Stored: {:x}", attributes_val);
    assert_ne!(attributes_val & FILE_ATTRIBUTE_HIDDEN, 0, "Hidden attribute (0x2) was not stored. Stored: {:x}", attributes_val);

    // Extract and verify restored attributes
    let output_dir_multi = temp_dir.path().join("output_multi");
    fs::create_dir(&output_dir_multi)?;

    StdCommand::pna()
        .arg("extract")
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&output_dir_multi)
        .arg("--restore-windows-attributes")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let extracted_file_path = output_dir_multi.join(test_file_name);
    assert!(extracted_file_path.exists());
    check_windows_file_attributes(&extracted_file_path, true, true)?;

    Ok(())
}
