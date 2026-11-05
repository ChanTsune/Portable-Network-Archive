use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, FileEntryBuilder};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn build_single_file_archive(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    writer
        .add_entry({
            let mut builder = FileEntryBuilder::new("test.txt".into()).unwrap();
            builder.write_all(b"file content").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer.finalize().unwrap();
}

fn extract(archive: &Path, out_dir: &Path, flags: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--extract",
        "--file",
        archive.to_str().unwrap(),
        "--out-dir",
        out_dir.to_str().unwrap(),
    ])
    .args(flags)
    .assert()
}

fn assert_extracted(out_dir: &Path) {
    assert_eq!(
        "file content",
        fs::read_to_string(out_dir.join("test.txt")).unwrap()
    );
    let mut names: Vec<_> = fs::read_dir(out_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["test.txt"]);
}

/// Precondition: Archive contains a single file.
/// Action: Extract via bsdtar without either safe-writes flag.
/// Expectation: Extraction succeeds and only the archived file is extracted.
#[test]
fn bsdtar_extract_without_safe_writes_flags() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_default");
    let _ = fs::remove_dir_all(&root);
    let archive = root.join("archive.pna");
    let out_dir = root.join("out");
    build_single_file_archive(&archive);

    extract(&archive, &out_dir, &[]).success();

    assert_extracted(&out_dir);
}

/// Precondition: Archive contains a single file.
/// Action: Extract via bsdtar with `--safe-writes`.
/// Expectation: Extraction succeeds and only the archived file is extracted;
/// no temporary file is left behind.
#[test]
fn bsdtar_extract_with_safe_writes() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_enabled");
    let _ = fs::remove_dir_all(&root);
    let archive = root.join("archive.pna");
    let out_dir = root.join("out");
    build_single_file_archive(&archive);

    extract(&archive, &out_dir, &["--safe-writes"]).success();

    assert_extracted(&out_dir);
}

/// Precondition: Archive contains a single file.
/// Action: Extract via bsdtar with `--no-safe-writes`.
/// Expectation: Extraction succeeds and only the archived file is extracted.
#[test]
fn bsdtar_extract_with_no_safe_writes() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_disabled");
    let _ = fs::remove_dir_all(&root);
    let archive = root.join("archive.pna");
    let out_dir = root.join("out");
    build_single_file_archive(&archive);

    extract(&archive, &out_dir, &["--no-safe-writes"]).success();

    assert_extracted(&out_dir);
}

/// Precondition: Archive contains a single file.
/// Action: Extract via bsdtar with both `--safe-writes` and `--no-safe-writes`.
/// Expectation: The command fails and nothing is extracted.
#[test]
fn bsdtar_extract_safe_writes_flags_conflict() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_conflict");
    let _ = fs::remove_dir_all(&root);
    let archive = root.join("archive.pna");
    let out_dir = root.join("out");
    build_single_file_archive(&archive);

    extract(&archive, &out_dir, &["--safe-writes", "--no-safe-writes"]).failure();

    assert!(!out_dir.join("test.txt").exists());
}

/// Precondition: Archive contains a single file.
/// Action: Extract via bsdtar with both `--no-safe-writes` and `--safe-writes`
/// in reverse order.
/// Expectation: The command fails and nothing is extracted.
#[test]
fn bsdtar_extract_safe_writes_flags_conflict_reversed() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_conflict_reversed");
    let _ = fs::remove_dir_all(&root);
    let archive = root.join("archive.pna");
    let out_dir = root.join("out");
    build_single_file_archive(&archive);

    extract(&archive, &out_dir, &["--no-safe-writes", "--safe-writes"]).failure();

    assert!(!out_dir.join("test.txt").exists());
}

/// Precondition: A directory contains a file to archive.
/// Action: Create an archive via bsdtar with both `--safe-writes` and
/// `--no-safe-writes`.
/// Expectation: The command fails and no archive is created.
#[test]
fn bsdtar_create_safe_writes_flags_conflict() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_create_conflict");
    let _ = fs::remove_dir_all(&root);
    let in_dir = root.join("in");
    fs::create_dir_all(&in_dir).unwrap();
    fs::write(in_dir.join("test.txt"), "file content").unwrap();
    let archive = root.join("archive.pna");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--file",
        archive.to_str().unwrap(),
        "--cd",
        in_dir.to_str().unwrap(),
        "--safe-writes",
        "--no-safe-writes",
        "test.txt",
    ])
    .assert()
    .failure();

    assert!(!archive.exists());
}
