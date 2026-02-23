use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Helper macro to set permissions, skipping the test if permission denied.
#[cfg(unix)]
macro_rules! set_permissions_or_skip {
    ($path:expr, $mode:expr) => {
        match fs::set_permissions($path, fs::Permissions::from_mode($mode)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                eprintln!(
                    "Skipping test: insufficient permissions to set file permissions: {}",
                    e
                );
                return;
            }
            Err(e) => panic!("Failed to set permissions: {}", e),
        }
    };
}

/// Recursively remove a directory, restoring write permissions on files if needed.
/// This is necessary because files with 0o000 permissions cannot be overwritten.
#[cfg(unix)]
fn force_remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }

    // Walk the directory and restore write permissions on all files
    fn restore_permissions(dir: &std::path::Path) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                restore_permissions(&path)?;
            } else if file_type.is_file() {
                // Try to restore write permission so the file can be deleted
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
            }
        }
        Ok(())
    }

    // Ignore errors during permission restoration - we'll try to remove anyway
    let _ = restore_permissions(path);
    fs::remove_dir_all(path)
}

/// Precondition: An archive contains a file with permission 0o755 (rwxr-xr-x).
/// Action: Extract the archive with `--keep-permission`.
/// Expectation: The extracted file has permission 0o755 on the filesystem.
#[test]
#[cfg(unix)]
fn extract_preserves_executable_permission() {
    setup();
    TestResources::extract_in("raw/", "extract_perm_755/in/").unwrap();

    set_permissions_or_skip!("extract_perm_755/in/raw/text.txt", 0o755);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_perm_755/archive.pna",
        "--overwrite",
        "extract_perm_755/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("extract_perm_755/archive.pna", |entry| {
        if entry.header().path().as_str().ends_with("raw/text.txt") {
            let perm = entry.metadata().permission().unwrap();
            assert_eq!(perm.permissions() & 0o777, 0o755);
        }
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_perm_755/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_perm_755/out/",
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::symlink_metadata("extract_perm_755/out/raw/text.txt").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o755,
        "extracted file should have permission 0o755"
    );
}

/// Precondition: An archive contains a file with permission 0o644 (rw-r--r--).
/// Action: Extract the archive with `--keep-permission`.
/// Expectation: The extracted file has permission 0o644 on the filesystem.
#[test]
#[cfg(unix)]
fn extract_preserves_readonly_permission() {
    setup();
    TestResources::extract_in("raw/", "extract_perm_644/in/").unwrap();

    set_permissions_or_skip!("extract_perm_644/in/raw/text.txt", 0o644);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_perm_644/archive.pna",
        "--overwrite",
        "extract_perm_644/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("extract_perm_644/archive.pna", |entry| {
        if entry.header().path().as_str().ends_with("raw/text.txt") {
            let perm = entry.metadata().permission().unwrap();
            assert_eq!(perm.permissions() & 0o777, 0o644);
        }
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_perm_644/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_perm_644/out/",
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::symlink_metadata("extract_perm_644/out/raw/text.txt").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o644,
        "extracted file should have permission 0o644"
    );
}

/// Precondition: An archive contains a file with permission 0o600 (rw-------).
/// Action: Extract the archive with `--keep-permission`.
/// Expectation: The extracted file has permission 0o600 on the filesystem.
#[test]
#[cfg(unix)]
fn extract_preserves_private_permission() {
    setup();
    TestResources::extract_in("raw/", "extract_perm_600/in/").unwrap();

    set_permissions_or_skip!("extract_perm_600/in/raw/text.txt", 0o600);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_perm_600/archive.pna",
        "--overwrite",
        "extract_perm_600/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("extract_perm_600/archive.pna", |entry| {
        if entry.header().path().as_str().ends_with("raw/text.txt") {
            let perm = entry.metadata().permission().unwrap();
            assert_eq!(perm.permissions() & 0o777, 0o600);
        }
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_perm_600/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_perm_600/out/",
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::symlink_metadata("extract_perm_600/out/raw/text.txt").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o600,
        "extracted file should have permission 0o600"
    );
}

/// Precondition: An archive contains a file with permission 0o000 (---------).
/// Action: Extract the archive with `--keep-permission`.
/// Expectation: The extracted file has permission 0o000 on the filesystem.
#[test]
#[cfg(unix)]
fn extract_preserves_no_permission() {
    setup();
    // Clean up any leftover files from previous test runs.
    // Files with 0o000 permissions cannot be overwritten, so we must remove them first.
    let _ = force_remove_dir_all("extract_perm_000");
    TestResources::extract_in("raw/", "extract_perm_000/in/").unwrap();

    set_permissions_or_skip!("extract_perm_000/in/raw/text.txt", 0o000);

    // Creating an archive of a file with 0o000 permissions requires root
    // privileges to read the file. Skip the test if we can't read the file.
    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_perm_000/archive.pna",
        "--overwrite",
        "extract_perm_000/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute();

    if let Err(e) = &result {
        // Check if the error is permission-related (use {:#} to include full error chain)
        let full_error = format!("{:#}", e);
        if full_error.contains("Permission denied") || full_error.contains("permission denied") {
            eprintln!(
                "Skipping test: insufficient permissions to read file with 0o000 mode: {}",
                e
            );
            return;
        }
    }
    result.unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_perm_000/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_perm_000/out/",
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::symlink_metadata("extract_perm_000/out/raw/text.txt").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o000,
        "extracted file should have permission 0o000"
    );
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Extract the archive with `--keep-permission`.
/// Expectation: The extracted file has permission 0o777 on the filesystem.
#[test]
#[cfg(unix)]
fn extract_preserves_full_permission() {
    setup();
    TestResources::extract_in("raw/", "extract_perm_777/in/").unwrap();

    set_permissions_or_skip!("extract_perm_777/in/raw/text.txt", 0o777);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_perm_777/archive.pna",
        "--overwrite",
        "extract_perm_777/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("extract_perm_777/archive.pna", |entry| {
        if entry.header().path().as_str().ends_with("raw/text.txt") {
            let perm = entry.metadata().permission().unwrap();
            assert_eq!(perm.permissions() & 0o777, 0o777);
        }
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_perm_777/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_perm_777/out/",
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::symlink_metadata("extract_perm_777/out/raw/text.txt").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o777,
        "extracted file should have permission 0o777"
    );
}

/// Precondition: An archive contains multiple files with different permissions.
/// Action: Extract the archive with `--keep-permission`.
/// Expectation: Each extracted file has its original permission preserved.
#[test]
#[cfg(unix)]
fn extract_preserves_mixed_permissions() {
    setup();
    TestResources::extract_in("raw/", "extract_perm_mixed/in/").unwrap();

    // Set different permissions for different files
    set_permissions_or_skip!("extract_perm_mixed/in/raw/text.txt", 0o755);
    set_permissions_or_skip!("extract_perm_mixed/in/raw/empty.txt", 0o644);
    set_permissions_or_skip!("extract_perm_mixed/in/raw/images/icon.png", 0o600);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_perm_mixed/archive.pna",
        "--overwrite",
        "extract_perm_mixed/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("extract_perm_mixed/archive.pna", |entry| {
        if entry.header().path().as_str().ends_with("raw/text.txt") {
            assert_eq!(
                entry.metadata().permission().unwrap().permissions() & 0o777,
                0o755
            );
        } else if entry.header().path().as_str().ends_with("raw/empty.txt") {
            assert_eq!(
                entry.metadata().permission().unwrap().permissions() & 0o777,
                0o644
            );
        } else if entry
            .header()
            .path()
            .as_str()
            .ends_with("raw/images/icon.png")
        {
            assert_eq!(
                entry.metadata().permission().unwrap().permissions() & 0o777,
                0o600
            );
        }
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_perm_mixed/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_perm_mixed/out/",
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::symlink_metadata("extract_perm_mixed/out/raw/text.txt").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o755,
        "text.txt should have permission 0o755"
    );

    let meta = fs::symlink_metadata("extract_perm_mixed/out/raw/empty.txt").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o644,
        "empty.txt should have permission 0o644"
    );

    let meta = fs::symlink_metadata("extract_perm_mixed/out/raw/images/icon.png").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o600,
        "icon.png should have permission 0o600"
    );
}

/// Precondition: An archive contains a directory with permission 0o750 (rwxr-x---).
/// Action: Extract with `--keep-permission`.
/// Expectation: The extracted directory has permission 0o750 on the filesystem.
#[test]
#[cfg(unix)]
fn extract_preserves_directory_permission() {
    setup();
    TestResources::extract_in("raw/", "extract_dir_perm/in/").unwrap();

    set_permissions_or_skip!("extract_dir_perm/in/raw/images", 0o750);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_dir_perm/archive.pna",
        "--overwrite",
        "extract_dir_perm/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("extract_dir_perm/archive.pna", |entry| {
        if entry.header().path().as_str().ends_with("raw/images") {
            assert_eq!(entry.header().data_kind(), pna::DataKind::Directory);
            let perm = entry.metadata().permission().unwrap();
            assert_eq!(perm.permissions() & 0o777, 0o750);
        }
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_dir_perm/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_dir_perm/out/",
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::symlink_metadata("extract_dir_perm/out/raw/images").unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o750,);
}
