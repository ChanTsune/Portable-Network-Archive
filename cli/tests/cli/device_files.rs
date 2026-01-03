//! Integration tests for device file support (block devices, character devices, FIFOs).
//!
//! These tests verify that device files can be archived and extracted correctly.
//! Note: Creating block/character devices requires root privileges and may be
//! restricted in containerized environments.

use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, path::Path};

/// Creates a FIFO in the specified directory.
/// Returns true if successful, false otherwise.
#[cfg(unix)]
fn create_fifo(path: &Path) -> bool {
    use nix::sys::stat::Mode;
    use nix::unistd;

    if path.exists() {
        let _ = fs::remove_file(path);
    }
    unistd::mkfifo(path, Mode::from_bits_truncate(0o644)).is_ok()
}

/// Creates a character device in the specified directory.
/// Returns true if successful, false otherwise (e.g., not root or restricted environment).
#[cfg(unix)]
fn create_char_device(path: &Path, major: u32, minor: u32) -> bool {
    use nix::sys::stat::{self, Mode, SFlag};

    if path.exists() {
        let _ = fs::remove_file(path);
    }
    let dev = libc::makedev(major, minor);
    stat::mknod(path, SFlag::S_IFCHR, Mode::from_bits_truncate(0o666), dev).is_ok()
}

/// Creates a block device in the specified directory.
/// Returns true if successful, false otherwise (e.g., not root or restricted environment).
#[cfg(unix)]
fn create_block_device(path: &Path, major: u32, minor: u32) -> bool {
    use nix::sys::stat::{self, Mode, SFlag};

    if path.exists() {
        let _ = fs::remove_file(path);
    }
    let dev = libc::makedev(major, minor);
    stat::mknod(path, SFlag::S_IFBLK, Mode::from_bits_truncate(0o660), dev).is_ok()
}

/// Precondition: A FIFO is created in the input directory.
/// Action: Archive the FIFO and extract it.
/// Expectation: The extracted file is a FIFO.
#[test]
#[cfg(unix)]
fn fifo_roundtrip() {
    use std::os::unix::fs::FileTypeExt;

    setup();

    let base_path = Path::new("device_files_fifo");
    let in_path = base_path.join("in");
    let archive_path = base_path.join("archive.pna");
    let out_path = base_path.join("out");

    // Clean up from previous runs
    if base_path.exists() {
        fs::remove_dir_all(base_path).unwrap();
    }
    fs::create_dir_all(&in_path).unwrap();

    // Create test FIFO
    let fifo_path = in_path.join("test_fifo");
    if !create_fifo(&fifo_path) {
        eprintln!("Skipping fifo_roundtrip: cannot create FIFO");
        return;
    }

    // Create archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "--file",
        &archive_path.to_string_lossy(),
        "--overwrite",
        "--keep-permission",
        &in_path.to_string_lossy(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(archive_path.exists());

    // Extract archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "extract",
        "--file",
        &archive_path.to_string_lossy(),
        "--overwrite",
        "--out-dir",
        &out_path.to_string_lossy(),
        "--keep-permission",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify FIFO was extracted correctly
    let extracted_fifo = out_path.join("in/test_fifo");
    assert!(
        extracted_fifo.exists(),
        "FIFO should exist after extraction"
    );

    let metadata = fs::symlink_metadata(&extracted_fifo).unwrap();
    assert!(
        metadata.file_type().is_fifo(),
        "Extracted file should be a FIFO"
    );
}

/// Precondition: Block and character devices are created (requires root and mknod capability).
/// Action: Archive the devices and extract them.
/// Expectation: The extracted files are devices with correct major/minor numbers.
#[test]
#[cfg(unix)]
fn device_nodes_roundtrip() {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    setup();

    let base_path = Path::new("device_files_nodes");
    let in_path = base_path.join("in");
    let archive_path = base_path.join("archive.pna");
    let out_path = base_path.join("out");

    // Clean up from previous runs
    if base_path.exists() {
        fs::remove_dir_all(base_path).unwrap();
    }
    fs::create_dir_all(&in_path).unwrap();

    // Try to create test devices (may fail in containers)
    let char_path = in_path.join("test_char");
    let block_path = in_path.join("test_block");

    let char_created = create_char_device(&char_path, 1, 3);
    let block_created = create_block_device(&block_path, 7, 0);

    if !char_created && !block_created {
        eprintln!(
            "Skipping device_nodes_roundtrip: cannot create device nodes (requires root and mknod capability)"
        );
        return;
    }

    // Create archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "--file",
        &archive_path.to_string_lossy(),
        "--overwrite",
        "--keep-permission",
        &in_path.to_string_lossy(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(archive_path.exists());

    // Extract archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "extract",
        "--file",
        &archive_path.to_string_lossy(),
        "--overwrite",
        "--out-dir",
        &out_path.to_string_lossy(),
        "--keep-permission",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify character device (if it was created)
    if char_created {
        let extracted_char = out_path.join("in/test_char");
        assert!(
            extracted_char.exists(),
            "Character device should exist after extraction"
        );
        let metadata = fs::symlink_metadata(&extracted_char).unwrap();
        assert!(
            metadata.file_type().is_char_device(),
            "Extracted file should be a character device"
        );
        // Verify major/minor (1, 3 for /dev/null-like device)
        let rdev = metadata.rdev();
        let major = libc::major(rdev);
        let minor = libc::minor(rdev);
        assert_eq!(major, 1, "Character device major number should be 1");
        assert_eq!(minor, 3, "Character device minor number should be 3");
    }

    // Verify block device (if it was created)
    if block_created {
        let extracted_block = out_path.join("in/test_block");
        assert!(
            extracted_block.exists(),
            "Block device should exist after extraction"
        );
        let metadata = fs::symlink_metadata(&extracted_block).unwrap();
        assert!(
            metadata.file_type().is_block_device(),
            "Extracted file should be a block device"
        );
        // Verify major/minor (7, 0 for loop device)
        let rdev = metadata.rdev();
        let major = libc::major(rdev);
        let minor = libc::minor(rdev);
        assert_eq!(major, 7, "Block device major number should be 7");
        assert_eq!(minor, 0, "Block device minor number should be 0");
    }
}
