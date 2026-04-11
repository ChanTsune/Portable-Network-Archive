use crate::utils::{archive, setup};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::{fs, path::PathBuf};

/// Precondition: Archive contains a symlink entry with permission metadata.
/// Action: Extract with `--keep-permission`.
/// Expectation: The symlink is extracted successfully and points to the correct target.
#[test]
fn extract_symlink_preserves_permission_in_archive() {
    setup();
    let base = "extract_symlink_preserves_permission_in_archive";
    let archive_path = format!("{base}/{base}.pna");
    fs::create_dir_all(base).unwrap();

    archive::create_archive_with_symlinks(
        &archive_path,
        &[archive::FileEntryDef {
            path: "target.txt",
            content: b"content",
            permission: 0o644,
        }],
        &[archive::SymlinkEntryDef {
            path: "link.txt",
            target: "target.txt",
            permission: Some(0o755),
            modified: None,
            accessed: None,
            created: None,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &format!("{base}/out"),
        "--keep-permission",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link_path = PathBuf::from(format!("{base}/out/link.txt"));
    assert!(link_path.is_symlink(), "extracted path should be a symlink");
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("target.txt"),
        "symlink should point to the correct target"
    );
}

/// Precondition: Archive contains a broken symlink entry (target does not exist) with permission metadata.
/// Action: Extract with `--keep-permission`.
/// Expectation: The broken symlink is extracted with the correct link target.
#[test]
fn extract_broken_symlink_preserves_target_path() {
    setup();
    let base = "extract_broken_symlink_preserves_target_path";
    let archive_path = format!("{base}/{base}.pna");
    fs::create_dir_all(base).unwrap();

    archive::create_archive_with_symlinks(
        &archive_path,
        &[],
        &[archive::SymlinkEntryDef {
            path: "broken.txt",
            target: "nonexistent",
            permission: Some(0o777),
            modified: None,
            accessed: None,
            created: None,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &format!("{base}/out"),
        "--keep-permission",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link_path = PathBuf::from(format!("{base}/out/broken.txt"));
    assert!(
        link_path.is_symlink(),
        "broken symlink should be extracted as a symlink"
    );
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("nonexistent"),
        "broken symlink should point to the original (nonexistent) target"
    );
}

/// Precondition: Archive contains a broken symlink entry with a known mtime.
/// Action: Extract with `--keep-timestamp`.
/// Expectation: The broken symlink is extracted successfully with the correct link target.
#[test]
fn extract_broken_symlink_with_keep_timestamp() {
    setup();
    let base = "extract_broken_symlink_with_keep_timestamp";
    let archive_path = format!("{base}/{base}.pna");
    fs::create_dir_all(base).unwrap();

    let epoch_2024 = Duration::seconds(1_704_067_200);

    archive::create_archive_with_symlinks(
        &archive_path,
        &[],
        &[archive::SymlinkEntryDef {
            path: "broken.txt",
            target: "nonexistent",
            permission: Some(0o777),
            modified: Some(epoch_2024),
            accessed: None,
            created: None,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &format!("{base}/out"),
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link_path = PathBuf::from(format!("{base}/out/broken.txt"));
    assert!(link_path.is_symlink(), "broken symlink should be extracted");
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("nonexistent"),
    );
}

/// Precondition: Archive contains a symlink entry with permission and timestamp metadata.
/// Action: Extract with all preservation flags enabled.
/// Expectation: Extraction succeeds, the symlink exists correctly, and the target content is intact.
#[test]
fn extract_symlink_with_all_preservation_flags() {
    setup();
    let base = "extract_symlink_with_all_preservation_flags";
    let archive_path = format!("{base}/{base}.pna");
    fs::create_dir_all(base).unwrap();

    let epoch = Duration::seconds(1_704_067_200);

    archive::create_archive_with_symlinks(
        &archive_path,
        &[archive::FileEntryDef {
            path: "target.txt",
            content: b"content",
            permission: 0o644,
        }],
        &[archive::SymlinkEntryDef {
            path: "link.txt",
            target: "target.txt",
            permission: Some(0o777),
            modified: Some(epoch),
            accessed: Some(epoch),
            created: Some(epoch),
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &format!("{base}/out"),
        "--keep-permission",
        "--unstable",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link_path = PathBuf::from(format!("{base}/out/link.txt"));
    assert!(link_path.is_symlink(), "extracted path should be a symlink");
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("target.txt"),
    );
    // Verify the target file was not corrupted by the symlink extraction
    let target_content = fs::read_to_string(format!("{base}/out/target.txt")).unwrap();
    assert_eq!(target_content, "content");
}
