#![cfg(not(target_family = "wasm"))]
use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::{collections::HashSet, fs};

/// Precondition: Files exist on disk.
/// Action: Create archive using old-style syntax (cvf).
/// Expectation: Archive is created with correct entries.
#[test]
fn stdio_create_old_style_cvf() {
    setup();
    let dir = "old_style_cvf_dir";
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{dir}/a.txt"), "hello").unwrap();
    fs::write(format!("{dir}/b.txt"), "world").unwrap();

    let archive_path = "old_style_cvf.pna";

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "stdio",
            "cvf",
            archive_path,
            &format!("{dir}/a.txt"),
            &format!("{dir}/b.txt"),
        ])
        .assert()
        .success();

    let mut entries = HashSet::new();
    archive::for_each_entry(archive_path, |entry| {
        entries.insert(entry.header().path().to_string());
    })
    .unwrap();
    let expected: HashSet<String> = [format!("{dir}/a.txt"), format!("{dir}/b.txt")].into();
    assert_eq!(entries, expected);
}

/// Precondition: Archive exists with known entries.
/// Action: Extract using old-style syntax (xf).
/// Expectation: Files are extracted to disk.
#[test]
fn stdio_extract_old_style_xf() {
    setup();
    let src = "old_style_xf_src";
    fs::create_dir_all(src).unwrap();
    fs::write(format!("{src}/file.txt"), "data").unwrap();

    let archive_path = "old_style_xf.pna";

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "stdio",
            "cf",
            archive_path,
            &format!("{src}/file.txt"),
        ])
        .assert()
        .success();

    fs::remove_dir_all(src).unwrap();

    let out_dir = "old_style_xf_out";
    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "stdio",
            "xf",
            archive_path,
            "--out-dir",
            out_dir,
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(format!("{out_dir}/{src}/file.txt")).unwrap(),
        "data"
    );
}

/// Precondition: Archive exists with entries.
/// Action: List contents using old-style syntax (tf).
/// Expectation: Entry names are shown in output.
#[test]
fn stdio_list_old_style_tf() {
    setup();
    let file = "old_style_tf_file.txt";
    fs::write(file, "content").unwrap();

    let archive_path = "old_style_tf.pna";

    cargo_bin_cmd!("pna")
        .args(["experimental", "stdio", "cf", archive_path, file])
        .assert()
        .success();

    let output = cargo_bin_cmd!("pna")
        .args(["experimental", "stdio", "tf", archive_path])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(file));
}
