use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, WriteOptions, fs as pna_fs};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn build_single_file_archive(path: &PathBuf, name: &str, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file(name.into(), WriteOptions::builder().build()).unwrap();
            builder.write_all(contents.as_bytes()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer.finalize().unwrap();
}

#[test]
fn unlink_first_replaces_existing_symlink_file() {
    setup();

    let root = PathBuf::from("stdio_unlink_first_file");
    let archive_path = root.join("archive.pna");
    build_single_file_archive(&archive_path, "file.txt", "updated");

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let outside = root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("original.txt"), "keep me").unwrap();

    let link_path = dist.join("file.txt");
    if link_path.exists() {
        pna_fs::remove_path_all(&link_path).unwrap();
    }
    pna_fs::symlink(outside.join("original.txt"), &link_path).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--unlink-first",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(outside.join("original.txt")).unwrap(),
        "keep me"
    );
    assert_eq!(
        fs::read_to_string(dist.join("file.txt")).unwrap(),
        "updated"
    );
    assert!(
        !fs::symlink_metadata(dist.join("file.txt"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn unlink_first_removes_symlinked_parent_directory() {
    setup();

    let root = PathBuf::from("stdio_unlink_first_parent");
    let archive_path = root.join("archive.pna");
    build_single_file_archive(&archive_path, "dir/file.txt", "payload");

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let outside = root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("marker.txt"), "preserve").unwrap();

    let link_dir = dist.join("dir");
    if link_dir.exists() {
        pna_fs::remove_path_all(&link_dir).unwrap();
    }
    pna_fs::symlink(&outside, &link_dir).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--unlink-first",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    let extracted_dir_meta = fs::symlink_metadata(&link_dir).unwrap();
    assert!(extracted_dir_meta.is_dir());
    assert!(!extracted_dir_meta.file_type().is_symlink());
    assert!(fs::read(dist.join("dir/file.txt")).unwrap() == b"payload");
    assert_eq!(
        fs::read_to_string(outside.join("marker.txt")).unwrap(),
        "preserve"
    );
}

#[test]
fn unlink_first_replaces_existing_regular_file_without_overwrite() {
    setup();

    let root = PathBuf::from("stdio_unlink_first_regular");
    let archive_path = root.join("archive.pna");
    build_single_file_archive(&archive_path, "file.txt", "fresh");

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    fs::write(dist.join("file.txt"), "stale").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--unlink-first",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(dist.join("file.txt")).unwrap(), "fresh");
}
