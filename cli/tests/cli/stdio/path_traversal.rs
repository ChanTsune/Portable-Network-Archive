use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, EntryName, EntryReference, WriteOptions};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn build_archive_with_file(archive_path: &Path, file_name: &str, file_content: &[u8]) {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(archive_path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    writer
        .add_entry({
            let mut builder = EntryBuilder::new_file(
                EntryName::from_utf8_preserve_root(file_name),
                WriteOptions::builder().build(),
            )
            .unwrap();
            builder.write_all(file_content).unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

fn build_archive_with_file_and_symlink(
    archive_path: &Path,
    file_name: &str,
    file_content: &[u8],
    symlink_name: &str,
    symlink_target: &str,
) {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(archive_path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    writer
        .add_entry({
            let mut builder = EntryBuilder::new_file(
                EntryName::from_utf8_preserve_root(file_name),
                WriteOptions::builder().build(),
            )
            .unwrap();
            builder.write_all(file_content).unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer
        .add_entry({
            EntryBuilder::new_symlink(
                EntryName::from_utf8_preserve_root(symlink_name),
                EntryReference::from_utf8_preserve_root(symlink_target),
            )
            .unwrap()
            .build()
            .unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

fn build_archive_with_file_and_hardlink(
    archive_path: &Path,
    file_name: &str,
    file_content: &[u8],
    hardlink_name: &str,
    hardlink_target: &str,
) {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(archive_path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    writer
        .add_entry({
            let mut builder = EntryBuilder::new_file(
                EntryName::from_utf8_preserve_root(file_name),
                WriteOptions::builder().build(),
            )
            .unwrap();
            builder.write_all(file_content).unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer
        .add_entry({
            EntryBuilder::new_hard_link(
                EntryName::from_utf8_preserve_root(hardlink_name),
                EntryReference::from_utf8_preserve_root(hardlink_target),
            )
            .unwrap()
            .build()
            .unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

// --- Symlink target defense ---
// Note: stdio mode defaults to allow_unsafe_links=true (bsdtar-compatible).
// Use --no-allow-unsafe-links to enable blocking.

/// Precondition: Archive contains a symlink whose target uses parent directory traversal (..)
/// Action: Extract with stdio -x and --no-allow-unsafe-links
/// Expectation: Symlink is not created (blocked by unsafe link check)
#[test]
fn stdio_extract_blocks_symlink_with_parent_dir_target() {
    setup();

    let root = PathBuf::from("stdio_extract_blocks_symlink_parent");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_symlink(
        &archive_path,
        "./b/file.txt",
        b"content",
        "./a/link",
        "../b/file.txt",
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--no-allow-unsafe-links",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        out_dir.join("b/file.txt").exists(),
        "regular file should be extracted"
    );
    assert!(
        !out_dir.join("a/link").exists(),
        "symlink with .. target should be blocked by --no-allow-unsafe-links"
    );
}

/// Precondition: Archive contains a symlink whose target uses parent directory traversal (..)
/// Action: Extract with stdio -x (default: unsafe links allowed, bsdtar-compatible)
/// Expectation: Symlink is created with the original target preserved
#[test]
fn stdio_extract_allows_symlink_with_parent_dir_target_by_default() {
    setup();

    let root = PathBuf::from("stdio_extract_allows_symlink_parent_default");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_symlink(
        &archive_path,
        "./b/file.txt",
        b"content",
        "./a/link",
        "../b/file.txt",
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let link_path = out_dir.join("a/link");
    let meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(meta.file_type().is_symlink(), "symlink should be created");

    let target = fs::read_link(&link_path).unwrap();
    assert_eq!(
        target,
        Path::new("../b/file.txt"),
        "symlink target should preserve .. (bsdtar-compatible default)"
    );
}

/// Precondition: Archive contains a symlink with an absolute target path
/// Action: Extract with stdio -x (default: unsafe links allowed)
/// Expectation: Symlink is created with absolute target preserved (bsdtar passes verbatim)
#[test]
fn stdio_extract_symlink_with_absolute_target_preserved() {
    setup();

    let root = PathBuf::from("stdio_extract_symlink_abs_preserved_default");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_symlink(
        &archive_path,
        "./a/file.txt",
        b"content",
        "./a/link",
        "/etc/hostname",
    );

    // Default stdio: allow_unsafe_links=true, symlink targets passed verbatim (bsdtar-compat)
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let link_path = out_dir.join("a/link");
    let meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(meta.file_type().is_symlink(), "symlink should be created");

    let target = fs::read_link(&link_path).unwrap();
    assert_eq!(
        target,
        Path::new("/etc/hostname"),
        "absolute symlink target should be preserved verbatim (bsdtar-compat)"
    );
}

/// Precondition: Archive contains a symlink with an absolute target path
/// Action: Extract with stdio -x and --absolute-paths (-P)
/// Expectation: Symlink is created with the absolute target preserved
#[test]
fn stdio_extract_symlink_with_absolute_target_preserved_with_absolute_paths() {
    setup();

    let root = PathBuf::from("stdio_extract_symlink_abs_preserved");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_symlink(
        &archive_path,
        "./a/file.txt",
        b"content",
        "./a/link",
        "/etc/hostname",
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--absolute-paths",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let link_path = out_dir.join("a/link");
    let meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(meta.file_type().is_symlink(), "symlink should be created");

    let target = fs::read_link(&link_path).unwrap();
    assert_eq!(
        target,
        Path::new("/etc/hostname"),
        "absolute symlink target should be preserved with -P"
    );
}

// --- Hardlink target defense ---

/// Precondition: Archive contains a hardlink whose target has .. that resolves within out_dir
/// Action: Extract with stdio -x and --no-allow-unsafe-links
/// Expectation: Hardlink is not created (blocked by unsafe link check)
#[test]
fn stdio_extract_blocks_hardlink_with_parent_dir_target() {
    setup();

    let root = PathBuf::from("stdio_extract_blocks_hardlink_parent");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    // Target ./a/../a/file.txt has .. but resolves to ./a/file.txt within out_dir
    build_archive_with_file_and_hardlink(
        &archive_path,
        "./a/file.txt",
        b"content",
        "./link.txt",
        "./a/../a/file.txt",
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--no-allow-unsafe-links",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        out_dir.join("a/file.txt").exists(),
        "regular file should be extracted"
    );
    assert!(
        !out_dir.join("link.txt").exists(),
        "hardlink with .. in target should be blocked by --no-allow-unsafe-links"
    );
}

/// Precondition: Archive contains a hardlink whose target has .. traversal
/// Action: Extract with stdio -x (default: SECURE_NODOTDOT enabled)
/// Expectation: Hardlink is not created (rejected by NODOTDOT)
#[test]
fn stdio_extract_rejects_hardlink_with_dotdot_by_default() {
    setup();

    let root = PathBuf::from("stdio_extract_rejects_hardlink_dotdot_default");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_hardlink(
        &archive_path,
        "./a/file.txt",
        b"content",
        "./link.txt",
        "./a/../a/file.txt",
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        out_dir.join("a/file.txt").exists(),
        "regular file should be extracted"
    );
    assert!(
        !out_dir.join("link.txt").exists(),
        "hardlink with .. should be rejected by SECURE_NODOTDOT"
    );
}

/// Precondition: Archive contains a hardlink whose target has .. traversal
/// Action: Extract with stdio -x and --absolute-paths (-P disables NODOTDOT)
/// Expectation: Hardlink is created (.. resolves at filesystem level)
#[test]
fn stdio_extract_allows_hardlink_with_dotdot_with_absolute_paths() {
    setup();

    let root = PathBuf::from("stdio_extract_allows_hardlink_dotdot_abs");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_hardlink(
        &archive_path,
        "./a/file.txt",
        b"content",
        "./link.txt",
        "./a/../a/file.txt",
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--absolute-paths",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        out_dir.join("link.txt").exists(),
        "hardlink should be created (-P disables NODOTDOT)"
    );
    assert!(
        same_file::is_same_file(out_dir.join("a/file.txt"), out_dir.join("link.txt")).unwrap(),
        "hardlink should share inode with target"
    );
}

// --- Safe links work normally ---

/// Precondition: Archive contains a symlink with a safe relative target (no ..)
/// Action: Extract with stdio -x (default settings)
/// Expectation: Symlink is created normally
#[test]
fn stdio_extract_symlink_with_safe_relative_target() {
    setup();

    let root = PathBuf::from("stdio_extract_symlink_safe");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_symlink(
        &archive_path,
        "./a/b/file.txt",
        b"content",
        "./a/link",
        "b/file.txt",
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let link_path = out_dir.join("a/link");
    let meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(
        meta.file_type().is_symlink(),
        "safe symlink should be created"
    );

    let target = fs::read_link(&link_path).unwrap();
    assert_eq!(target, Path::new("b/file.txt"));
}

/// Precondition: Archive contains a hardlink with a safe target (no .., no /)
/// Action: Extract with stdio -x (default settings)
/// Expectation: Hardlink is created and shares inode with the target
#[test]
fn stdio_extract_hardlink_with_safe_target() {
    setup();

    let root = PathBuf::from("stdio_extract_hardlink_safe");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_hardlink(
        &archive_path,
        "./file.txt",
        b"content",
        "./link.txt",
        "./file.txt",
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out_dir.join("link.txt").exists(), "hardlink should exist");
    assert_eq!(
        fs::read_to_string(out_dir.join("link.txt")).unwrap(),
        "content"
    );
    assert!(
        same_file::is_same_file(out_dir.join("file.txt"), out_dir.join("link.txt")).unwrap(),
        "hardlink should share inode with target"
    );
}

// --- SECURE_NODOTDOT: entry names with ".." ---

/// Precondition: Archive contains a file whose entry name includes ".." traversal
/// Action: Extract with stdio -x (default: SECURE_NODOTDOT enabled)
/// Expectation: Entry is skipped (rejected by NODOTDOT check)
#[test]
fn stdio_extract_rejects_entry_with_dotdot() {
    setup();

    let root = PathBuf::from("stdio_extract_rejects_entry_dotdot");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file(&archive_path, "./a/../b/file.txt", b"content");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        !out_dir.join("b/file.txt").exists(),
        "entry with .. should be rejected by SECURE_NODOTDOT"
    );
    assert!(
        !out_dir.join("a/../b/file.txt").exists(),
        "entry with .. should not be extracted at all"
    );
}

/// Precondition: Archive contains a file whose entry name includes ".." traversal
/// Action: Extract with stdio -x and --absolute-paths (-P disables NODOTDOT)
/// Expectation: Entry is extracted (.. resolves at filesystem level)
#[test]
fn stdio_extract_allows_entry_with_dotdot_with_absolute_paths() {
    setup();

    let root = PathBuf::from("stdio_extract_allows_entry_dotdot_abs");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file(&archive_path, "./a/../b/file.txt", b"content");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--absolute-paths",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        out_dir.join("b/file.txt").exists(),
        "entry should be extracted (.. resolved by filesystem)"
    );
}
