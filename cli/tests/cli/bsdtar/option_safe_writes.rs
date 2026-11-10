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
/// Expectation: Both flags are accepted and only the archived file is extracted.
#[test]
fn bsdtar_extract_with_both_safe_writes_flags() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_both");
    let _ = fs::remove_dir_all(&root);
    let archive = root.join("archive.pna");
    let out_dir = root.join("out");
    build_single_file_archive(&archive);

    extract(&archive, &out_dir, &["--safe-writes", "--no-safe-writes"]).success();

    assert_extracted(&out_dir);
}

/// Precondition: Archive contains a single file.
/// Action: Extract via bsdtar with both `--no-safe-writes` and `--safe-writes`
/// in reverse order.
/// Expectation: Both flags are accepted and only the archived file is extracted.
#[test]
fn bsdtar_extract_with_both_safe_writes_flags_reversed() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_both_reversed");
    let _ = fs::remove_dir_all(&root);
    let archive = root.join("archive.pna");
    let out_dir = root.join("out");
    build_single_file_archive(&archive);

    extract(&archive, &out_dir, &["--no-safe-writes", "--safe-writes"]).success();

    assert_extracted(&out_dir);
}

/// Precondition: A directory contains a file to archive.
/// Action: Create an archive via bsdtar with both `--safe-writes` and
/// `--no-safe-writes`, then extract the created archive.
/// Expectation: Both flags are accepted, the archive is created, and the
/// archived file is extracted from it.
#[test]
fn bsdtar_create_with_both_safe_writes_flags() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_create_both");
    let _ = fs::remove_dir_all(&root);
    let in_dir = root.join("in");
    fs::create_dir_all(&in_dir).unwrap();
    fs::write(in_dir.join("test.txt"), "file content").unwrap();
    let archive = root.join("archive.pna");
    let out_dir = root.join("out");

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
    .success();

    extract(&archive, &out_dir, &[]).success();
    assert_extracted(&out_dir);
}

/// Precondition: An encrypted archive entry would replace an existing file.
/// Action: Extract with an incorrect password and opposing safe-writes flags in both orders.
/// Expectation: The last flag determines whether a failed extraction preserves the existing file.
#[test]
fn bsdtar_extract_safe_writes_flags_use_last_occurrence() {
    setup();
    let root = PathBuf::from("bsdtar_safe_writes_last_occurrence");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let archive = root.join("archive.pna");
    let options = pna::WriteOptions::builder()
        .encryption(pna::Encryption::AES)
        .cipher_mode(pna::CipherMode::GCM)
        .hash_algorithm(pna::HashAlgorithm::pbkdf2_sha256_with(Some(1000)))
        .password(Some("correct"))
        .build();
    let mut writer = Archive::write_header(fs::File::create(&archive).unwrap()).unwrap();
    let mut entry = FileEntryBuilder::new_with_options("test.txt".into(), &options).unwrap();
    entry.write_all(b"new content").unwrap();
    writer.add_entry(entry.build().unwrap()).unwrap();
    writer.finalize().unwrap();

    let safe = root.join("safe");
    fs::create_dir_all(&safe).unwrap();
    fs::write(safe.join("test.txt"), "original").unwrap();
    extract(
        &archive,
        &safe,
        &[
            "--password=wrong",
            "--no-safe-writes",
            "--safe-writes",
            "--safe-writes",
        ],
    )
    .failure();
    assert_eq!(fs::read(safe.join("test.txt")).unwrap(), b"original");

    let unsafe_dir = root.join("unsafe");
    fs::create_dir_all(&unsafe_dir).unwrap();
    fs::write(unsafe_dir.join("test.txt"), "original").unwrap();
    extract(
        &archive,
        &unsafe_dir,
        &[
            "--password=wrong",
            "--safe-writes",
            "--no-safe-writes",
            "--no-safe-writes",
        ],
    )
    .failure();
    assert_eq!(fs::read(unsafe_dir.join("test.txt")).unwrap(), b"");
}
