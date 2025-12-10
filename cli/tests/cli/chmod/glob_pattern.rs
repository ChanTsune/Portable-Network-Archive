use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains multiple .txt files with permission 0o644.
/// Action: Run `pna experimental chmod` with glob pattern `**/*.txt` and mode `755`.
/// Expectation: All .txt files have permission 0o755 in the archive; other files are unchanged.
#[test]
fn chmod_glob_pattern_txt_files() {
    setup();
    TestResources::extract_in("raw/", "chmod_glob_txt/in/").unwrap();

    // Set initial permissions for all txt files
    #[cfg(unix)]
    {
        fs::set_permissions(
            "chmod_glob_txt/in/raw/text.txt",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        fs::set_permissions(
            "chmod_glob_txt/in/raw/empty.txt",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        fs::set_permissions(
            "chmod_glob_txt/in/raw/parent/child.txt",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        fs::set_permissions(
            "chmod_glob_txt/in/raw/first/second/third/pna.txt",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        // Set non-txt file to different permission
        fs::set_permissions(
            "chmod_glob_txt/in/raw/images/icon.png",
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_glob_txt/archive.pna",
        "--overwrite",
        "chmod_glob_txt/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_glob_txt/archive.pna",
        "755",
        "**/*.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive metadata directly
    archive::for_each_entry("chmod_glob_txt/archive.pna", |entry| {
        let path = entry.header().path();
        let path_str = path.as_str();
        if let Some(perm) = entry.metadata().permission() {
            if path_str.ends_with(".txt") {
                assert_eq!(
                    perm.permissions() & 0o777,
                    0o755,
                    "{} should be 755",
                    path_str
                );
            } else if path_str == "chmod_glob_txt/in/raw/images/icon.png" {
                assert_eq!(
                    perm.permissions() & 0o777,
                    0o600,
                    "icon.png should remain 600"
                );
            }
        }
    })
    .unwrap();
}

/// Precondition: An archive contains multiple files with various permissions.
/// Action: Run `pna experimental chmod` targeting multiple explicit files.
/// Expectation: Only the specified files have their permissions changed in the archive.
#[test]
fn chmod_multiple_explicit_files() {
    setup();
    TestResources::extract_in("raw/", "chmod_multi_files/in/").unwrap();

    #[cfg(unix)]
    {
        fs::set_permissions(
            "chmod_multi_files/in/raw/text.txt",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        fs::set_permissions(
            "chmod_multi_files/in/raw/empty.txt",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        fs::set_permissions(
            "chmod_multi_files/in/raw/images/icon.png",
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_multi_files/archive.pna",
        "--overwrite",
        "chmod_multi_files/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Change permissions on two specific files
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_multi_files/archive.pna",
        "755",
        "chmod_multi_files/in/raw/text.txt",
        "chmod_multi_files/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive metadata directly
    archive::for_each_entry("chmod_multi_files/archive.pna", |entry| {
        let path = entry.header().path();
        let path_str = path.as_str();
        if let Some(perm) = entry.metadata().permission() {
            if path_str == "chmod_multi_files/in/raw/text.txt"
                || path_str == "chmod_multi_files/in/raw/empty.txt"
            {
                assert_eq!(
                    perm.permissions() & 0o777,
                    0o755,
                    "{} should be 755",
                    path_str
                );
            } else if path_str == "chmod_multi_files/in/raw/images/icon.png" {
                assert_eq!(
                    perm.permissions() & 0o777,
                    0o600,
                    "icon.png should remain 600"
                );
            }
        }
    })
    .unwrap();
}

/// Precondition: An archive contains files in nested directories.
/// Action: Run `pna experimental chmod` with pattern targeting a subdirectory.
/// Expectation: Only files in the matching subdirectory have changed permissions in the archive.
#[test]
fn chmod_glob_pattern_subdirectory() {
    setup();
    TestResources::extract_in("raw/", "chmod_glob_subdir/in/").unwrap();

    #[cfg(unix)]
    {
        // Set all image files to 644
        fs::set_permissions(
            "chmod_glob_subdir/in/raw/images/icon.png",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        fs::set_permissions(
            "chmod_glob_subdir/in/raw/images/icon.svg",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        fs::set_permissions(
            "chmod_glob_subdir/in/raw/images/icon.bmp",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        // Set a file outside images to 644 as well
        fs::set_permissions(
            "chmod_glob_subdir/in/raw/text.txt",
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_glob_subdir/archive.pna",
        "--overwrite",
        "chmod_glob_subdir/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Change permissions only for files in images directory
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_glob_subdir/archive.pna",
        "755",
        "**/images/*",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive metadata directly
    archive::for_each_entry("chmod_glob_subdir/archive.pna", |entry| {
        let path = entry.header().path();
        let path_str = path.as_str();
        if let Some(perm) = entry.metadata().permission() {
            if path_str.contains("/images/") {
                assert_eq!(
                    perm.permissions() & 0o777,
                    0o755,
                    "{} should be 755",
                    path_str
                );
            } else if path_str == "chmod_glob_subdir/in/raw/text.txt" {
                assert_eq!(
                    perm.permissions() & 0o777,
                    0o644,
                    "text.txt should remain 644"
                );
            }
        }
    })
    .unwrap();
}

/// Precondition: An archive contains files, and we verify permissions via archive inspection.
/// Action: Run `pna experimental chmod` and check archive entry metadata directly.
/// Expectation: Archive entries have the correct permission metadata.
#[test]
fn chmod_verify_archive_metadata() {
    setup();
    TestResources::extract_in("raw/", "chmod_verify_meta/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_verify_meta/in/raw/text.txt",
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_verify_meta/archive.pna",
        "--overwrite",
        "chmod_verify_meta/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_verify_meta/archive.pna",
        "755",
        "chmod_verify_meta/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify the archive entry metadata directly
    archive::for_each_entry("chmod_verify_meta/archive.pna", |entry| {
        if entry.header().path() == "chmod_verify_meta/in/raw/text.txt" {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o755,
                "archive entry should have 755"
            );
        }
    })
    .unwrap();
}
