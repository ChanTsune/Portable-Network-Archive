use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

/// Precondition: An archive contains multiple .txt files with permission 0o644.
/// Action: Run `pna experimental chmod` with glob pattern `**/*.txt` and mode `755`.
/// Expectation: All .txt files have permission 0o755 in the archive; other files are unchanged.
#[test]
fn chmod_glob_pattern_txt_files() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_glob_txt.pna",
        &[
            FileEntryDef {
                path: "dir/text.txt",
                content: b"text content",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/empty.txt",
                content: b"",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/sub/child.txt",
                content: b"child content",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/images/icon.png",
                content: b"png data",
                permission: 0o600,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_glob_txt.pna",
        "755",
        "**/*.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_glob_txt.pna", |entry| {
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
            } else if path_str.ends_with(".png") {
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

    archive::create_archive_with_permissions(
        "chmod_multi_files.pna",
        &[
            FileEntryDef {
                path: "dir/text.txt",
                content: b"text content",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/empty.txt",
                content: b"",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/images/icon.png",
                content: b"png data",
                permission: 0o600,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_multi_files.pna",
        "755",
        "dir/text.txt",
        "dir/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_multi_files.pna", |entry| {
        let path = entry.header().path();
        let path_str = path.as_str();
        if let Some(perm) = entry.metadata().permission() {
            if path_str == "dir/text.txt" || path_str == "dir/empty.txt" {
                assert_eq!(
                    perm.permissions() & 0o777,
                    0o755,
                    "{} should be 755",
                    path_str
                );
            } else if path_str.ends_with(".png") {
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

    archive::create_archive_with_permissions(
        "chmod_glob_subdir.pna",
        &[
            FileEntryDef {
                path: "dir/images/icon.png",
                content: b"png data",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/images/icon.svg",
                content: b"svg data",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/text.txt",
                content: b"text content",
                permission: 0o644,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_glob_subdir.pna",
        "755",
        "**/images/*",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_glob_subdir.pna", |entry| {
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
            } else if path_str.ends_with(".txt") {
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

    archive::create_archive_with_permissions(
        "chmod_verify_meta.pna",
        &[FileEntryDef {
            path: "test.txt",
            content: b"test content",
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_verify_meta.pna",
        "755",
        "test.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_verify_meta.pna", |entry| {
        if entry.header().path() == "test.txt" {
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
