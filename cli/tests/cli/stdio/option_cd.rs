use crate::utils::{
    archive::{create_test_archive, get_archive_entry_names},
    setup,
};
use assert_cmd::cargo::cargo_bin_cmd;
use std::collections::HashSet;
use std::fs;

/// Precondition: Archive source.pna contains file a.txt. Directory sub/ contains b.txt.
/// Action: Create with `@source.pna -C sub b.txt` -- archive inclusion before -C.
/// Expectation: Result contains both a.txt (from @source) and b.txt (from sub/).
///   @source.pna is resolved from original cwd, not from sub/.
#[test]
fn stdio_create_archive_inclusion_before_cd() {
    setup();

    let base = fs::canonicalize(".")
        .unwrap()
        .join("stdio_cd_archive_before_cd");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create source archive with a.txt
    let source_archive = base.join("source.pna");
    create_test_archive(&source_archive, &[("a.txt", "content a")]);

    // Create sub directory with b.txt
    let sub_dir = base.join("sub");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("b.txt"), "content b").unwrap();

    // Create result archive: @source.pna is before -C, so it resolves from base (cwd).
    // -C sub applies only to b.txt.
    let output_archive = base.join("result.pna");
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
            "-C",
            sub_dir.to_str().unwrap(),
            "b.txt",
        ])
        .assert()
        .success();

    // Verify the output archive contains both entries
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("a.txt"), "Missing a.txt from @source");
    assert!(
        entry_names.contains("b.txt"),
        "Missing b.txt from sub directory"
    );
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: Directories d1/ and d2/ each contain a file.
/// Action: Create with `-C d1 f1.txt -C d2 f2.txt` using absolute -C paths.
/// Expectation: Both files archived with base names only (no directory prefix).
#[test]
fn stdio_create_multiple_cd_absolute_paths() {
    setup();

    let base = fs::canonicalize(".")
        .unwrap()
        .join("stdio_cd_multiple_absolute");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create d1 with f1.txt
    let d1 = base.join("d1");
    fs::create_dir_all(&d1).unwrap();
    fs::write(d1.join("f1.txt"), "content f1").unwrap();

    // Create d2 with f2.txt
    let d2 = base.join("d2");
    fs::create_dir_all(&d2).unwrap();
    fs::write(d2.join("f2.txt"), "content f2").unwrap();

    let output_archive = base.join("result.pna");
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
            d1.to_str().unwrap(),
            "f1.txt",
            "-C",
            d2.to_str().unwrap(),
            "f2.txt",
        ])
        .assert()
        .success();

    // Verify both files are archived with base names
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("f1.txt"), "Missing f1.txt");
    assert!(entry_names.contains("f2.txt"), "Missing f2.txt");
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: A directory tree with nested files exists.
/// Action: Create with `-C` changing into different directories for each file group.
/// Expectation: Each file is archived relative to its -C directory.
#[test]
fn stdio_create_cd_does_not_affect_prior_args() {
    setup();

    let base = fs::canonicalize(".").unwrap().join("stdio_cd_ordering");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create directories
    let alpha = base.join("alpha");
    fs::create_dir_all(&alpha).unwrap();
    fs::write(alpha.join("first.txt"), "first").unwrap();

    let beta = base.join("beta");
    fs::create_dir_all(&beta).unwrap();
    fs::write(beta.join("second.txt"), "second").unwrap();

    // Archive first.txt from alpha, then second.txt from beta
    let output_archive = base.join("result.pna");
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
            alpha.to_str().unwrap(),
            "first.txt",
            "-C",
            beta.to_str().unwrap(),
            "second.txt",
        ])
        .assert()
        .success();

    // Verify entry order matches argument order
    let entry_names = get_archive_entry_names(&output_archive);
    assert_eq!(entry_names.len(), 2, "Expected exactly 2 entries");
    assert_eq!(entry_names[0], "first.txt");
    assert_eq!(entry_names[1], "second.txt");
}

/// Precondition: Archive source.pna exists in directory src/. File extra.txt exists in dir/.
/// Action: Create with `-C src @source.pna -C dir extra.txt`.
/// Expectation: @source.pna resolves from src/ (the active -C), extra.txt resolves from dir/.
#[test]
fn stdio_create_cd_affects_archive_inclusion() {
    setup();

    let base = fs::canonicalize(".")
        .unwrap()
        .join("stdio_cd_archive_resolution");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create source archive inside src/ subdirectory
    let src_dir = base.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let source_archive = src_dir.join("source.pna");
    create_test_archive(&source_archive, &[("archived.txt", "from archive")]);

    // Create extra file in dir/ subdirectory
    let dir = base.join("dir");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("extra.txt"), "extra content").unwrap();

    // -C src makes @source.pna resolve from src/
    // -C dir makes extra.txt resolve from dir/
    let output_archive = base.join("result.pna");
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
            src_dir.to_str().unwrap(),
            "@source.pna",
            "-C",
            dir.to_str().unwrap(),
            "extra.txt",
        ])
        .assert()
        .success();

    // Verify both entries present
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(
        entry_names.contains("archived.txt"),
        "Missing archived.txt from @source"
    );
    assert!(
        entry_names.contains("extra.txt"),
        "Missing extra.txt from dir/"
    );
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: An archive contains entries a.txt and b.txt.
/// Action: Extract with `-C <target_dir>` to redirect output.
/// Expectation: Files appear in the target directory with correct content.
#[test]
fn stdio_extract_with_cd() {
    setup();

    let base = fs::canonicalize(".").unwrap().join("stdio_extract_with_cd");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create archive with two entries
    let archive = base.join("test.pna");
    create_test_archive(&archive, &[("a.txt", "alpha"), ("b.txt", "beta")]);

    // Create target extraction directory
    let target = base.join("out");
    fs::create_dir_all(&target).unwrap();

    // Extract with -C pointing to target directory
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "-f",
            archive.to_str().unwrap(),
            "-C",
            target.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Verify files landed in the target directory
    assert_eq!(fs::read_to_string(target.join("a.txt")).unwrap(), "alpha");
    assert_eq!(fs::read_to_string(target.join("b.txt")).unwrap(), "beta");
}

/// Precondition: Archive contains a.txt. Directory sub/ contains b.txt.
/// Action: Update archive with `-C <sub> b.txt`.
/// Expectation: Archive contains both a.txt and b.txt.
#[test]
fn stdio_update_with_cd() {
    setup();

    let base = fs::canonicalize(".").unwrap().join("stdio_update_with_cd");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create initial archive with a.txt
    let archive = base.join("test.pna");
    create_test_archive(&archive, &[("a.txt", "content a")]);

    // Create sub directory with b.txt
    let sub_dir = base.join("sub");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("b.txt"), "content b").unwrap();

    // Update archive: -C sub b.txt
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--update",
            "--unstable",
            "-f",
            archive.to_str().unwrap(),
            "-C",
            sub_dir.to_str().unwrap(),
            "b.txt",
        ])
        .assert()
        .success();

    // Verify the archive now contains both entries
    let entry_names: HashSet<String> = get_archive_entry_names(&archive).into_iter().collect();
    assert!(entry_names.contains("a.txt"), "Missing a.txt");
    assert!(entry_names.contains("b.txt"), "Missing b.txt");
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: Archive contains a.txt. Directory sub/ contains b.txt.
/// Action: Append to archive with `-C <sub> b.txt`.
/// Expectation: Archive contains both a.txt and b.txt.
#[test]
fn stdio_append_with_cd() {
    setup();

    let base = fs::canonicalize(".").unwrap().join("stdio_append_with_cd");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create initial archive with a.txt
    let archive = base.join("test.pna");
    create_test_archive(&archive, &[("a.txt", "content a")]);

    // Create sub directory with b.txt
    let sub_dir = base.join("sub");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("b.txt"), "content b").unwrap();

    // Append to archive: -C sub b.txt
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--append",
            "--unstable",
            "-f",
            archive.to_str().unwrap(),
            "-C",
            sub_dir.to_str().unwrap(),
            "b.txt",
        ])
        .assert()
        .success();

    // Verify the archive now contains both entries
    let entry_names: HashSet<String> = get_archive_entry_names(&archive).into_iter().collect();
    assert!(entry_names.contains("a.txt"), "Missing a.txt");
    assert!(entry_names.contains("b.txt"), "Missing b.txt");
    assert_eq!(entry_names.len(), 2);
}
