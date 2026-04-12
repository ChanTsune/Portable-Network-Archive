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

/// Precondition: Archive contains a file (0o644) and a symlink (0o777) pointing to that file.
/// Action: Extract with `--keep-permission`.
/// Expectation: The target file retains its own permission (0o644); the symlink's permission
/// must not leak through to the target via chmod following the symlink.
#[cfg(unix)]
#[test]
fn extract_symlink_does_not_modify_target_permissions() {
    use std::os::unix::fs::PermissionsExt;

    setup();
    let base = "extract_symlink_does_not_modify_target_permissions";
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

    let target_path = PathBuf::from(format!("{base}/out/target.txt"));
    let target_meta = fs::metadata(&target_path).unwrap();
    let target_mode = target_meta.permissions().mode() & 0o777;
    assert_eq!(
        target_mode, 0o644,
        "target file permission should remain 0o644, not be changed by symlink's 0o755"
    );
}

/// Precondition: Archive contains a symlink entry with mtime=2024-01-01.
/// Action: Extract with `--keep-timestamp`.
/// Expectation: The symlink's mtime is restored to 2024-01-01.
#[test]
fn extract_symlink_with_keep_timestamp() {
    use std::time::SystemTime;

    setup();
    let base = "extract_symlink_with_keep_timestamp";
    let archive_path = format!("{base}/{base}.pna");
    fs::create_dir_all(base).unwrap();

    let epoch_2024 = Duration::seconds(1_704_067_200); // 2024-01-01T00:00:00Z

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
            modified: Some(epoch_2024),
            accessed: Some(epoch_2024),
            created: Some(epoch_2024),
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

    let link_path = PathBuf::from(format!("{base}/out/link.txt"));
    assert!(link_path.is_symlink(), "extracted path should be a symlink");
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("target.txt"),
    );
    let link_mtime = fs::symlink_metadata(&link_path)
        .unwrap()
        .modified()
        .unwrap();
    let expected = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_704_067_200);
    let diff = if link_mtime > expected {
        link_mtime.duration_since(expected).unwrap()
    } else {
        expected.duration_since(link_mtime).unwrap()
    };
    assert!(
        diff.as_secs() <= 1,
        "symlink mtime should be restored to 2024-01-01"
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
/// Expectation: The broken symlink is extracted with the correct target and its mtime is restored.
#[test]
fn extract_broken_symlink_with_keep_timestamp() {
    use std::time::SystemTime;

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
    let link_mtime = fs::symlink_metadata(&link_path)
        .unwrap()
        .modified()
        .unwrap();
    let expected = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_704_067_200);
    let diff = if link_mtime > expected {
        link_mtime.duration_since(expected).unwrap()
    } else {
        expected.duration_since(link_mtime).unwrap()
    };
    assert!(
        diff.as_secs() <= 1,
        "broken symlink mtime should be restored to the archived value"
    );
}

/// Precondition: Archive contains a symlink entry with mtime set but atime absent.
/// Action: Extract with `--keep-timestamp`.
/// Expectation: The symlink's mtime is restored; the existing atime is not clobbered to epoch.
#[test]
fn extract_symlink_with_partial_timestamps() {
    use std::time::SystemTime;

    setup();
    let base = "extract_symlink_with_partial_timestamps";
    let archive_path = format!("{base}/{base}.pna");
    fs::create_dir_all(base).unwrap();

    let epoch_2024 = Duration::seconds(1_704_067_200); // 2024-01-01T00:00:00Z

    // Create archive with symlink that has mtime but NO atime
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
            modified: Some(epoch_2024),
            accessed: None, // intentionally absent
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

    let link_path = PathBuf::from(format!("{base}/out/link.txt"));
    assert!(link_path.is_symlink());

    let link_meta = fs::symlink_metadata(&link_path).unwrap();

    // Verify mtime was restored to 2024-01-01
    let link_mtime = link_meta.modified().unwrap();
    let expected_mtime = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_704_067_200);
    let mtime_diff = if link_mtime > expected_mtime {
        link_mtime.duration_since(expected_mtime).unwrap()
    } else {
        expected_mtime.duration_since(link_mtime).unwrap()
    };
    assert!(
        mtime_diff.as_secs() <= 1,
        "symlink mtime should be restored to the archived value"
    );

    // Verify atime was NOT clobbered to Unix epoch (1970-01-01)
    let link_atime = link_meta.accessed().unwrap();
    let epoch_1970 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(86400);
    assert!(
        link_atime > epoch_1970,
        "atime should not be clobbered to Unix epoch when absent from archive"
    );
}

/// Precondition: Archive contains a symlink entry with all metadata (permission, timestamps).
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
