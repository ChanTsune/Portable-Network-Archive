//! Tests for permission preservation during archive extraction.
//!
//! These tests verify that file permissions stored in archives are correctly
//! applied to the filesystem when extracting with `--keep-permission`.

use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o755 (rwxr-xr-x).
/// Action: Extract the archive with `--keep-permission`.
/// Expectation: The extracted file has permission 0o755 on the filesystem.
#[test]
#[cfg(unix)]
fn extract_preserves_executable_permission() {
    setup();
    TestResources::extract_in("raw/", "extract_perm_755/in/").unwrap();

    fs::set_permissions(
        "extract_perm_755/in/raw/text.txt",
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

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

    fs::set_permissions(
        "extract_perm_644/in/raw/text.txt",
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

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

    fs::set_permissions(
        "extract_perm_600/in/raw/text.txt",
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();

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
    TestResources::extract_in("raw/", "extract_perm_000/in/").unwrap();

    fs::set_permissions(
        "extract_perm_000/in/raw/text.txt",
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_perm_000/archive.pna",
        "--overwrite",
        "extract_perm_000/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

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

    fs::set_permissions(
        "extract_perm_777/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

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
    fs::set_permissions(
        "extract_perm_mixed/in/raw/text.txt",
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::set_permissions(
        "extract_perm_mixed/in/raw/empty.txt",
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    fs::set_permissions(
        "extract_perm_mixed/in/raw/images/icon.png",
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();

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

/// Precondition: An archive contains files with permissions, created in solid mode.
/// Action: Extract the solid archive with `--keep-permission`.
/// Expectation: The extracted files have their original permissions preserved.
#[test]
#[cfg(unix)]
fn extract_preserves_permission_from_solid_archive() {
    setup();
    TestResources::extract_in("raw/", "extract_perm_solid/in/").unwrap();

    fs::set_permissions(
        "extract_perm_solid/in/raw/text.txt",
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_perm_solid/archive.pna",
        "--overwrite",
        "--solid",
        "extract_perm_solid/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_perm_solid/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_perm_solid/out/",
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::symlink_metadata("extract_perm_solid/out/raw/text.txt").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o755,
        "extracted file from solid archive should have permission 0o755"
    );
}

/// Precondition: An archive contains files with permissions and encryption.
/// Action: Extract the encrypted archive with `--keep-permission`.
/// Expectation: The extracted files have their original permissions preserved.
#[test]
#[cfg(unix)]
fn extract_preserves_permission_from_encrypted_archive() {
    setup();
    TestResources::extract_in("raw/", "extract_perm_enc/in/").unwrap();

    fs::set_permissions(
        "extract_perm_enc/in/raw/text.txt",
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_perm_enc/archive.pna",
        "--overwrite",
        "extract_perm_enc/in/",
        "--keep-permission",
        "--password",
        "testpass",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_perm_enc/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_perm_enc/out/",
        "--keep-permission",
        "--password",
        "testpass",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::symlink_metadata("extract_perm_enc/out/raw/text.txt").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o755,
        "extracted file from encrypted archive should have permission 0o755"
    );
}
