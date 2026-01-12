use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryBuilder, WriteOptions};
use portable_network_archive::cli;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn create_file_archive(archive_path: &Path, file_name: &str, content: &[u8]) {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    let mut builder =
        EntryBuilder::new_file(file_name.into(), WriteOptions::builder().build()).unwrap();
    builder.write_all(content).unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();
    archive.finalize().unwrap();
}

/// Precondition: Archive contains a regular file.
/// Action: Extract with `--safe-writes --unstable`.
/// Expectation: File is created with correct content.
#[test]
fn extract_with_safe_writes_creates_file() {
    setup();
    let archive_path = PathBuf::from("safe_writes/basic/archive.pna");
    let out_dir = PathBuf::from("safe_writes/basic/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    create_file_archive(&archive_path, "test.txt", b"content");
    fs::create_dir_all(&out_dir).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/basic/archive.pna",
        "--out-dir",
        "safe_writes/basic/out",
        "--safe-writes",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        fs::read_to_string(out_dir.join("test.txt")).unwrap(),
        "content"
    );
}

/// Precondition: Existing file at target, archive contains different content.
/// Action: Extract with `--safe-writes --unstable --overwrite`.
/// Expectation: File is atomically replaced with archive content.
#[test]
fn extract_with_safe_writes_replaces_existing_file() {
    setup();
    let archive_path = PathBuf::from("safe_writes/replace/archive.pna");
    let out_dir = PathBuf::from("safe_writes/replace/out");
    let target = out_dir.join("test.txt");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    create_file_archive(&archive_path, "test.txt", b"new content");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(&target, b"old content").unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "old content");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/replace/archive.pna",
        "--out-dir",
        "safe_writes/replace/out",
        "--safe-writes",
        "--unstable",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "new content");
}

/// Precondition: Empty directory at target path, archive contains file.
/// Action: Extract with `--safe-writes --unstable --overwrite`.
/// Expectation: Directory is replaced by file (libarchive-compatible behavior).
#[test]
fn extract_with_safe_writes_replaces_empty_directory() {
    setup();
    let archive_path = PathBuf::from("safe_writes/replace_dir/archive.pna");
    let out_dir = PathBuf::from("safe_writes/replace_dir/out");
    let target = out_dir.join("test.txt");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    create_file_archive(&archive_path, "test.txt", b"file content");
    fs::create_dir_all(&out_dir).unwrap();

    // Create empty directory at the target location
    fs::create_dir_all(&target).unwrap();
    assert!(target.is_dir());

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/replace_dir/archive.pna",
        "--out-dir",
        "safe_writes/replace_dir/out",
        "--safe-writes",
        "--unstable",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Should now be a file, not a directory
    assert!(target.is_file());
    assert_eq!(fs::read_to_string(&target).unwrap(), "file content");
}

/// Precondition: Non-empty directory at target path, archive contains file.
/// Action: Extract with `--safe-writes --unstable --overwrite`.
/// Expectation: Extraction fails (cannot rmdir non-empty directory).
#[test]
fn extract_with_safe_writes_fails_on_non_empty_directory() {
    setup();
    let archive_path = PathBuf::from("safe_writes/nonempty_dir/archive.pna");
    let out_dir = PathBuf::from("safe_writes/nonempty_dir/out");
    let target = out_dir.join("test.txt");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    create_file_archive(&archive_path, "test.txt", b"file content");
    fs::create_dir_all(&out_dir).unwrap();

    // Create non-empty directory at the target location
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("inside.txt"), "content inside").unwrap();
    assert!(target.is_dir());

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/nonempty_dir/archive.pna",
        "--out-dir",
        "safe_writes/nonempty_dir/out",
        "--safe-writes",
        "--unstable",
        "--overwrite",
    ])
    .unwrap()
    .execute();

    // Should fail because directory is not empty
    assert!(result.is_err());

    // Directory should still exist
    assert!(target.is_dir());
}

/// Precondition: Archive contains files.
/// Action: Extract with `--safe-writes --unstable`.
/// Expectation: No `.pna.*` temp files remain in output directory.
#[test]
fn extract_with_safe_writes_no_temp_files_on_success() {
    setup();
    let archive_path = PathBuf::from("safe_writes/no_temp/archive.pna");
    let out_dir = PathBuf::from("safe_writes/no_temp/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    create_file_archive(&archive_path, "test.txt", b"content");
    fs::create_dir_all(&out_dir).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/no_temp/archive.pna",
        "--out-dir",
        "safe_writes/no_temp/out",
        "--safe-writes",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Check no temp files remain
    for entry in fs::read_dir(&out_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        assert!(
            !name_str.starts_with(".pna."),
            "Temp file {} should have been cleaned up",
            name_str
        );
    }

    // Verify the actual file was extracted
    assert!(out_dir.join("test.txt").exists());
}

/// Precondition: Archive contains a symlink and its target file.
/// Action: Extract with `--safe-writes --unstable`.
/// Expectation: Symlink is correctly created pointing to target.
#[cfg(unix)]
#[test]
fn extract_with_safe_writes_creates_symlink() {
    setup();
    let archive_path = PathBuf::from("safe_writes/symlink/archive.pna");
    let out_dir = PathBuf::from("safe_writes/symlink/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    // Create archive with symlink and target file
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    // Add target file first
    let mut builder =
        EntryBuilder::new_file("target.txt".into(), WriteOptions::builder().build()).unwrap();
    builder.write_all(b"target content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    // Add symlink pointing to target
    let builder = EntryBuilder::new_symlink("link.txt".into(), "target.txt".into()).unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    fs::create_dir_all(&out_dir).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/symlink/archive.pna",
        "--out-dir",
        "safe_writes/symlink/out",
        "--safe-writes",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify symlink exists and is a symlink
    let link_path = out_dir.join("link.txt");
    assert!(
        fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink(),
        "link.txt should be a symlink"
    );

    // Verify symlink points to correct target
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("target.txt")
    );

    // Verify target file exists with correct content
    assert_eq!(
        fs::read_to_string(out_dir.join("target.txt")).unwrap(),
        "target content"
    );
}

/// Precondition: Archive contains a file and a hardlink to it.
/// Action: Extract with `--safe-writes --unstable`.
/// Expectation: Hardlink is correctly created sharing inode with original.
#[cfg(unix)]
#[test]
fn extract_with_safe_writes_creates_hardlink() {
    setup();
    let archive_path = PathBuf::from("safe_writes/hardlink/archive.pna");
    let out_dir = PathBuf::from("safe_writes/hardlink/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    // Create archive with file and hardlink
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    // Add original file
    let mut builder =
        EntryBuilder::new_file("original.txt".into(), WriteOptions::builder().build()).unwrap();
    builder.write_all(b"shared content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    // Add hardlink to original
    let builder =
        EntryBuilder::new_hard_link("hardlink.txt".into(), "original.txt".into()).unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    fs::create_dir_all(&out_dir).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/hardlink/archive.pna",
        "--out-dir",
        "safe_writes/hardlink/out",
        "--safe-writes",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let original_path = out_dir.join("original.txt");
    let hardlink_path = out_dir.join("hardlink.txt");

    // Verify both files exist with same content
    assert_eq!(
        fs::read_to_string(&original_path).unwrap(),
        "shared content"
    );
    assert_eq!(
        fs::read_to_string(&hardlink_path).unwrap(),
        "shared content"
    );

    // Verify they share the same inode (are hardlinks)
    #[cfg(not(target_family = "wasm"))]
    assert!(
        same_file::is_same_file(&original_path, &hardlink_path).unwrap(),
        "hardlink.txt should be a hardlink to original.txt"
    );
}

/// Precondition: Archive contains a regular file.
/// Action: Extract with `--no-safe-writes --unstable`.
/// Expectation: Extraction succeeds using normal (non-atomic) writes.
#[test]
fn extract_with_no_safe_writes_disables_atomic() {
    setup();
    let archive_path = PathBuf::from("safe_writes/no_safe_writes/archive.pna");
    let out_dir = PathBuf::from("safe_writes/no_safe_writes/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    create_file_archive(&archive_path, "test.txt", b"content");
    fs::create_dir_all(&out_dir).unwrap();

    // --no-safe-writes explicitly disables atomic extraction
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/no_safe_writes/archive.pna",
        "--out-dir",
        "safe_writes/no_safe_writes/out",
        "--no-safe-writes",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify file was extracted successfully
    assert_eq!(
        fs::read_to_string(out_dir.join("test.txt")).unwrap(),
        "content"
    );
}

/// Precondition: Both `--safe-writes` and `--no-safe-writes` provided.
/// Action: Attempt to parse CLI with conflicting flags.
/// Expectation: Argument parsing fails due to mutual exclusion.
#[test]
fn extract_safe_writes_flags_are_mutually_exclusive() {
    setup();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "dummy.pna",
        "--out-dir",
        "dummy",
        "--safe-writes",
        "--no-safe-writes",
        "--unstable",
    ]);

    // Should fail because flags are mutually exclusive
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("cannot be used with"),
        "Expected mutual exclusion error, got: {}",
        err_str
    );
}

/// Precondition: Archive contains a directory entry.
/// Action: Extract with `--safe-writes --unstable`.
/// Expectation: Directory is created normally (safe-writes only affects regular files).
#[test]
fn extract_with_safe_writes_creates_directory() {
    setup();
    let archive_path = PathBuf::from("safe_writes/directory/archive.pna");
    let out_dir = PathBuf::from("safe_writes/directory/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    // Create archive with directory entry
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let builder = EntryBuilder::new_dir("subdir".into());
    archive.add_entry(builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    fs::create_dir_all(&out_dir).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/directory/archive.pna",
        "--out-dir",
        "safe_writes/directory/out",
        "--safe-writes",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify directory was created
    let target = out_dir.join("subdir");
    assert!(target.is_dir(), "subdir should be a directory");
}

/// Precondition: Existing regular file at target, archive contains symlink.
/// Action: Extract with `--safe-writes --unstable --overwrite`.
/// Expectation: File is replaced by symlink.
#[cfg(unix)]
#[test]
fn extract_with_safe_writes_replaces_file_with_symlink() {
    setup();
    let archive_path = PathBuf::from("safe_writes/symlink_replace/archive.pna");
    let out_dir = PathBuf::from("safe_writes/symlink_replace/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    // Create archive with target file and symlink
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    // Add target file
    let mut builder =
        EntryBuilder::new_file("target.txt".into(), WriteOptions::builder().build()).unwrap();
    builder.write_all(b"target content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    // Add symlink
    let builder = EntryBuilder::new_symlink("link.txt".into(), "target.txt".into()).unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    fs::create_dir_all(&out_dir).unwrap();

    // Create existing regular file at symlink location
    let link_path = out_dir.join("link.txt");
    fs::write(&link_path, "old file content").unwrap();
    assert!(link_path.is_file());
    assert!(
        !fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/symlink_replace/archive.pna",
        "--out-dir",
        "safe_writes/symlink_replace/out",
        "--safe-writes",
        "--unstable",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify file was replaced by symlink
    assert!(
        fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink(),
        "link.txt should now be a symlink"
    );
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("target.txt")
    );
}

/// Precondition: Existing regular file at target, archive contains hardlink.
/// Action: Extract with `--safe-writes --unstable --overwrite`.
/// Expectation: File is replaced by hardlink.
#[cfg(unix)]
#[test]
fn extract_with_safe_writes_replaces_file_with_hardlink() {
    setup();
    let archive_path = PathBuf::from("safe_writes/hardlink_replace/archive.pna");
    let out_dir = PathBuf::from("safe_writes/hardlink_replace/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    // Create archive with original file and hardlink
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    // Add original file
    let mut builder =
        EntryBuilder::new_file("original.txt".into(), WriteOptions::builder().build()).unwrap();
    builder.write_all(b"shared content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    // Add hardlink
    let builder =
        EntryBuilder::new_hard_link("hardlink.txt".into(), "original.txt".into()).unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    fs::create_dir_all(&out_dir).unwrap();

    // Create existing regular file at hardlink location
    let hardlink_path = out_dir.join("hardlink.txt");
    fs::write(&hardlink_path, "old file content").unwrap();
    assert!(hardlink_path.is_file());

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/hardlink_replace/archive.pna",
        "--out-dir",
        "safe_writes/hardlink_replace/out",
        "--safe-writes",
        "--unstable",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let original_path = out_dir.join("original.txt");

    // Verify hardlink was created with correct content
    assert_eq!(
        fs::read_to_string(&hardlink_path).unwrap(),
        "shared content"
    );

    // Verify they share the same inode
    #[cfg(not(target_family = "wasm"))]
    assert!(
        same_file::is_same_file(&original_path, &hardlink_path).unwrap(),
        "hardlink.txt should be a hardlink to original.txt"
    );
}

/// Precondition: Existing symlink at target, archive contains regular file.
/// Action: Extract with `--safe-writes --unstable --overwrite`.
/// Expectation: Symlink is replaced by regular file.
#[cfg(unix)]
#[test]
fn extract_with_safe_writes_replaces_symlink_with_file() {
    setup();
    let archive_path = PathBuf::from("safe_writes/file_over_symlink/archive.pna");
    let out_dir = PathBuf::from("safe_writes/file_over_symlink/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    create_file_archive(&archive_path, "test.txt", b"new file content");
    fs::create_dir_all(&out_dir).unwrap();

    // Create a symlink at the target location
    let target_path = out_dir.join("test.txt");
    let dummy_target = out_dir.join("dummy_target");
    fs::write(&dummy_target, "dummy").unwrap();
    std::os::unix::fs::symlink(&dummy_target, &target_path).unwrap();
    assert!(
        fs::symlink_metadata(&target_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/file_over_symlink/archive.pna",
        "--out-dir",
        "safe_writes/file_over_symlink/out",
        "--safe-writes",
        "--unstable",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify symlink was replaced by regular file
    assert!(
        !fs::symlink_metadata(&target_path)
            .unwrap()
            .file_type()
            .is_symlink(),
        "test.txt should no longer be a symlink"
    );
    assert!(target_path.is_file());
    assert_eq!(
        fs::read_to_string(&target_path).unwrap(),
        "new file content"
    );
}

/// Precondition: Existing symlink at target, archive contains different symlink.
/// Action: Extract with `--safe-writes --unstable --overwrite`.
/// Expectation: Old symlink is replaced by new symlink.
#[cfg(unix)]
#[test]
fn extract_with_safe_writes_replaces_symlink_with_symlink() {
    setup();
    let archive_path = PathBuf::from("safe_writes/symlink_over_symlink/archive.pna");
    let out_dir = PathBuf::from("safe_writes/symlink_over_symlink/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    // Create archive with symlink pointing to "new_target.txt"
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let builder = EntryBuilder::new_symlink("link.txt".into(), "new_target.txt".into()).unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    fs::create_dir_all(&out_dir).unwrap();

    // Create existing symlink pointing to "old_target.txt"
    let link_path = out_dir.join("link.txt");
    std::os::unix::fs::symlink("old_target.txt", &link_path).unwrap();
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("old_target.txt")
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/symlink_over_symlink/archive.pna",
        "--out-dir",
        "safe_writes/symlink_over_symlink/out",
        "--safe-writes",
        "--unstable",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify symlink now points to new target
    assert!(
        fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("new_target.txt")
    );
}

/// Precondition: Existing symlink at target, archive contains hardlink.
/// Action: Extract with `--safe-writes --unstable --overwrite`.
/// Expectation: Symlink is replaced by hardlink.
#[cfg(unix)]
#[test]
fn extract_with_safe_writes_replaces_symlink_with_hardlink() {
    setup();
    let archive_path = PathBuf::from("safe_writes/hardlink_over_symlink/archive.pna");
    let out_dir = PathBuf::from("safe_writes/hardlink_over_symlink/out");

    // Clean up from previous runs
    let _ = fs::remove_dir_all(&out_dir);

    // Create archive with original file and hardlink
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    // Add original file
    let mut builder =
        EntryBuilder::new_file("original.txt".into(), WriteOptions::builder().build()).unwrap();
    builder.write_all(b"shared content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    // Add hardlink
    let builder = EntryBuilder::new_hard_link("link.txt".into(), "original.txt".into()).unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    fs::create_dir_all(&out_dir).unwrap();

    // Create existing symlink at hardlink location
    let link_path = out_dir.join("link.txt");
    std::os::unix::fs::symlink("some_target.txt", &link_path).unwrap();
    assert!(
        fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "safe_writes/hardlink_over_symlink/archive.pna",
        "--out-dir",
        "safe_writes/hardlink_over_symlink/out",
        "--safe-writes",
        "--unstable",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let original_path = out_dir.join("original.txt");

    // Verify symlink was replaced by hardlink (regular file)
    assert!(
        !fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink(),
        "link.txt should no longer be a symlink"
    );
    assert_eq!(fs::read_to_string(&link_path).unwrap(), "shared content");

    // Verify they share the same inode
    #[cfg(not(target_family = "wasm"))]
    assert!(
        same_file::is_same_file(&original_path, &link_path).unwrap(),
        "link.txt should be a hardlink to original.txt"
    );
}
