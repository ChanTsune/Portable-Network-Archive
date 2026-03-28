//! Integration tests for mtree format support in stdio commands.
//!
//! Tests verify that `@manifest.mtree` syntax works correctly for creating
//! archives from mtree manifest files.

use crate::utils::{archive::for_each_entry, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn get_archive_entry_names(path: &Path) -> Vec<String> {
    let mut names = Vec::new();
    for_each_entry(path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    names
}

/// Precondition: An mtree manifest specifies a single entry.
/// Action: Create archive from the mtree manifest.
/// Expectation: The archive contains exactly the entry specified in the manifest.
#[test]
fn stdio_mtree_basic_inclusion() {
    setup();

    let base = PathBuf::from("stdio_mtree_basic_inclusion");
    fs::create_dir_all(&base).unwrap();

    // Create source file
    fs::write(base.join("source.txt"), "file content").unwrap();

    // Create mtree manifest
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\nentry.txt contents=source.txt\n",
    )
    .unwrap();

    // Create archive via stdio
    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .success();

    // Verify archive contents
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("entry.txt"), "Missing entry.txt");
    assert_eq!(entry_names.len(), 1);
}

/// Precondition: An mtree manifest uses /set directive for default metadata.
/// Action: Create archive from the mtree manifest.
/// Expectation: The archive contains all entries with correct metadata applied.
#[test]
fn stdio_mtree_with_set_directive() {
    setup();

    let base = PathBuf::from("stdio_mtree_with_set_directive");
    fs::create_dir_all(&base).unwrap();

    fs::write(base.join("app.txt"), "application").unwrap();
    fs::write(base.join("config.txt"), "configuration").unwrap();

    // Create mtree with /set directive
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\n/set type=file mode=0644\napp.txt mode=0755\nconfig.txt\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("app.txt"));
    assert!(entry_names.contains("config.txt"));
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: An mtree manifest uses contents= to specify alternate file source.
/// Action: Create archive from the mtree manifest.
/// Expectation: The entry path is from manifest, content is from the referenced file.
#[test]
fn stdio_mtree_contents_keyword() {
    setup();

    let base = PathBuf::from("stdio_mtree_contents_keyword");
    fs::create_dir_all(base.join("build")).unwrap();

    // Create source file in different location
    fs::write(base.join("build/compiled.bin"), "binary content").unwrap();

    // Create mtree with contents= pointing to different path
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\nusr/bin/app contents=build/compiled.bin\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    // Entry should be named usr/bin/app, not build/compiled.bin
    assert!(entry_names.contains("usr/bin/app"), "Missing usr/bin/app");
    assert_eq!(entry_names.len(), 1);
}

/// Precondition: An mtree manifest uses CRLF line endings, wrapped lines, and `content=` alias.
/// Action: Create and extract archive from the mtree manifest.
/// Expectation: Parsing succeeds and entries are created with expected payloads.
#[test]
fn stdio_mtree_crlf_wrapped_and_content_alias() {
    setup();

    let base = PathBuf::from("stdio_mtree_crlf_wrapped_and_content_alias");
    fs::create_dir_all(base.join("bar")).unwrap();
    fs::write(base.join("bar/foo"), "abc").unwrap();
    fs::write(base.join("bar/goo"), "xyz").unwrap();

    fs::write(
        base.join("manifest.mtree"),
        "#mtree\r\nf type=file uname=\\\r\nroot gname=root mode=0755 content=bar/foo\r\ng type=file uname=root gname=root mode=0755 content=bar/goo\r\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("f"), "Missing f");
    assert!(entry_names.contains("g"), "Missing g");
    assert_eq!(entry_names.len(), 2);

    let out_dir = base.join("out");
    fs::create_dir_all(&out_dir).unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(fs::read(out_dir.join("f")).unwrap(), b"abc");
    assert_eq!(fs::read(out_dir.join("g")).unwrap(), b"xyz");
}

/// Precondition: An mtree manifest specifies directory and file entries.
/// Action: Create archive from the mtree manifest.
/// Expectation: The archive contains both directory and file entries.
#[test]
fn stdio_mtree_directory_entry() {
    setup();

    let base = PathBuf::from("stdio_mtree_directory_entry");
    fs::create_dir_all(base.join("subdir")).unwrap();
    fs::write(base.join("subdir/file.txt"), "nested file").unwrap();

    // Create mtree with directory type
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\nsubdir type=dir\nsubdir/file.txt type=file\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("subdir"));
    assert!(entry_names.contains("subdir/file.txt"));
}

/// Precondition: An mtree manifest specifies symlink entries with link= keyword.
/// Action: Create archive from the mtree manifest.
/// Expectation: The archive contains the symlink entries.
#[cfg(unix)]
#[test]
fn stdio_mtree_symlink_entry() {
    setup();

    let base = PathBuf::from("stdio_mtree_symlink_entry");
    // Clean up from previous runs (symlinks cause AlreadyExists errors)
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    // Create target file and symlink
    fs::write(base.join("target.txt"), "target content").unwrap();
    std::os::unix::fs::symlink("target.txt", base.join("link.txt")).unwrap();

    // Create mtree with symlink entry
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\ntarget.txt type=file\nlink.txt type=link link=target.txt\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("target.txt"));
    assert!(entry_names.contains("link.txt"));
}

/// Precondition: Both an mtree manifest and standalone files exist.
/// Action: Create archive including both @manifest.mtree and regular files.
/// Expectation: The archive contains entries from both sources.
#[test]
fn stdio_mtree_with_filesystem_files() {
    setup();

    let base = PathBuf::from("stdio_mtree_with_filesystem_files");
    fs::create_dir_all(&base).unwrap();

    // Create files for mtree
    fs::write(base.join("mtree_file.txt"), "from mtree").unwrap();

    // Create standalone file (not in mtree)
    fs::write(base.join("standalone.txt"), "standalone file").unwrap();

    // Create mtree manifest
    fs::write(base.join("manifest.mtree"), "#mtree\nmtree_file.txt\n").unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "standalone.txt",
            "@manifest.mtree",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    // Both standalone file and mtree entry should be in archive
    assert!(entry_names.contains("standalone.txt"));
    assert!(entry_names.contains("mtree_file.txt"));
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: An mtree manifest references a nonexistent file without optional keyword.
/// Action: Attempt to create archive from the mtree manifest.
/// Expectation: The command fails with an error.
#[test]
fn stdio_mtree_missing_required_file_fails() {
    setup();

    let base = PathBuf::from("stdio_mtree_missing_required_file_fails");
    fs::create_dir_all(&base).unwrap();

    // Create mtree referencing nonexistent file (not optional)
    fs::write(base.join("manifest.mtree"), "#mtree\nnonexistent.txt\n").unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .failure();
}

/// Precondition: An mtree manifest references a nonexistent file WITH optional keyword.
/// Action: Create archive from the mtree manifest.
/// Expectation: Command succeeds, missing optional entry is skipped.
#[test]
fn stdio_mtree_optional_file_skipped() {
    setup();

    let base = PathBuf::from("stdio_mtree_optional_file_skipped");
    fs::create_dir_all(&base).unwrap();

    // Create only the required file, not the optional one
    fs::write(base.join("exists.txt"), "content").unwrap();

    // Create mtree with both required and optional entries
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\nexists.txt\nmissing.txt optional\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .success();

    // Verify: only exists.txt is in archive, missing.txt is skipped
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("exists.txt"));
    assert!(!entry_names.contains("missing.txt"));
    assert_eq!(entry_names.len(), 1);
}

/// Precondition: Both a PNA archive and an mtree manifest exist.
/// Action: Create archive including both @pna_archive and @manifest.mtree.
/// Expectation: Format detection correctly distinguishes PNA from mtree.
#[test]
fn stdio_format_detection_pna_vs_mtree() {
    setup();

    let base = PathBuf::from("stdio_format_detection_pna_vs_mtree");
    fs::create_dir_all(&base).unwrap();

    // Create source files
    fs::write(base.join("from_pna.txt"), "content from pna").unwrap();
    fs::write(base.join("from_mtree.txt"), "content from mtree").unwrap();

    // Create a PNA archive with one file
    let pna_archive = base.join("source.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            pna_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "from_pna.txt",
        ])
        .assert()
        .success();

    // Create mtree manifest referencing another file
    fs::write(base.join("manifest.mtree"), "#mtree\nfrom_mtree.txt\n").unwrap();

    // Create final archive using both @pna and @mtree
    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@source.pna",
            "@manifest.mtree",
        ])
        .assert()
        .success();

    // Verify both entries are present
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(
        entry_names.contains("from_pna.txt"),
        "Missing from_pna.txt (PNA detection failed)"
    );
    assert!(
        entry_names.contains("from_mtree.txt"),
        "Missing from_mtree.txt (mtree detection failed)"
    );
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: An mtree file contains invalid syntax.
/// Action: Attempt to create archive from the invalid mtree.
/// Expectation: Command fails with an error.
#[test]
fn stdio_mtree_parse_error_invalid_syntax() {
    setup();

    let base = PathBuf::from("stdio_mtree_parse_error_invalid_syntax");
    fs::create_dir_all(&base).unwrap();

    // Create mtree with invalid syntax (unbalanced quotes, invalid keywords)
    fs::write(
        base.join("invalid.mtree"),
        "#mtree\nfile.txt mode=\"invalid\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@invalid.mtree",
        ])
        .assert()
        .failure();
}

/// Precondition: mtree specifies mode=0755 with nochange keyword, file has mode 0644.
/// Action: Create archive (stdio stores permissions by default).
/// Expectation: Archived entry has filesystem mode (0644), not mtree value.
#[test]
#[cfg(unix)]
fn stdio_mtree_nochange_uses_filesystem_metadata() {
    use std::os::unix::fs::PermissionsExt;

    setup();

    let base = PathBuf::from("stdio_mtree_nochange_uses_filesystem_metadata");
    fs::create_dir_all(&base).unwrap();

    // Create file with specific mode (0644)
    let file_path = base.join("file.txt");
    fs::write(&file_path, "test content").unwrap();
    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();

    // Create mtree with nochange keyword and different mode (0755)
    // nochange means: use filesystem metadata, ignore mtree-specified values
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\nfile.txt mode=0755 nochange\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .success();

    // Verify: entry should have filesystem mode (0644), not mtree mode (0755)
    let mut found = false;
    for_each_entry(&output_archive, |entry| {
        if entry.header().path().as_str() == "file.txt" {
            let permission = entry.metadata().permission().expect("permission not set");
            // nochange should cause filesystem mode (0644) to be used, not mtree mode (0755)
            assert_eq!(
                permission.permissions() & 0o777,
                0o644,
                "nochange should use filesystem mode, not mtree mode"
            );
            found = true;
        }
    })
    .unwrap();
    assert!(found, "entry not found");
}

/// Precondition: mtree specifies type=file but path is a directory.
/// Action: Create archive from the mtree.
/// Expectation: Command fails with error indicating type mismatch.
#[test]
fn stdio_mtree_type_mismatch_file_is_dir() {
    setup();

    let base = PathBuf::from("stdio_mtree_type_mismatch_file_is_dir");
    fs::create_dir_all(base.join("actually_a_dir")).unwrap();

    // Create mtree claiming the directory is a file
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\nactually_a_dir type=file\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .failure();
}

/// Precondition: mtree specifies type=dir but path is a regular file.
/// Action: Create archive from the mtree.
/// Expectation: Command fails with error indicating type mismatch.
#[test]
fn stdio_mtree_type_mismatch_dir_is_file() {
    setup();

    let base = PathBuf::from("stdio_mtree_type_mismatch_dir_is_file");
    fs::create_dir_all(&base).unwrap();

    // Create a regular file
    fs::write(base.join("actually_a_file.txt"), "content").unwrap();

    // Create mtree claiming the file is a directory
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\nactually_a_file.txt type=dir\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .failure();
}

/// Precondition: mtree specifies type=link but path is a regular file.
/// Action: Create archive from the mtree.
/// Expectation: Command fails with error indicating type mismatch.
#[test]
fn stdio_mtree_type_mismatch_link_is_file() {
    setup();

    let base = PathBuf::from("stdio_mtree_type_mismatch_link_is_file");
    fs::create_dir_all(&base).unwrap();

    // Create a regular file
    fs::write(base.join("actually_a_file.txt"), "content").unwrap();

    // Create mtree claiming the file is a symlink
    fs::write(
        base.join("manifest.mtree"),
        "#mtree\nactually_a_file.txt type=link link=target.txt\n",
    )
    .unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@manifest.mtree",
        ])
        .assert()
        .failure();
}
