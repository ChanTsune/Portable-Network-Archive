use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, WriteOptions, fs as pna_fs};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn build_dir_and_file_archive(path: &PathBuf, dir_name: &str, file_name: &str, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    writer
        .add_entry(EntryBuilder::new_dir(dir_name.into()).build().unwrap())
        .unwrap();
    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file(file_name.into(), WriteOptions::builder().build()).unwrap();
            builder.write_all(contents.as_bytes()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer.finalize().unwrap();
}

#[test]
fn absolute_paths_keeps_existing_directory_symlink() {
    setup();

    let root = PathBuf::from("stdio_absolute_paths_keep_symlink");
    let archive_path = root.join("archive.pna");
    build_dir_and_file_archive(&archive_path, "dir", "dir/file.txt", "payload");

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let real_dir = root.join("real_dir");
    fs::create_dir_all(&real_dir).unwrap();
    let real_dir = fs::canonicalize(real_dir).unwrap();

    let link_dir = dist.join("dir");
    if link_dir.exists() {
        pna_fs::remove_path_all(&link_dir).unwrap();
    }
    pna_fs::symlink(&real_dir, &link_dir).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--absolute-paths",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = fs::symlink_metadata(&link_dir).unwrap();
    assert!(meta.file_type().is_symlink());
    assert_eq!(fs::read_link(&link_dir).unwrap(), real_dir);
    assert_eq!(
        fs::read_to_string(real_dir.join("file.txt")).unwrap(),
        "payload"
    );
}

#[test]
fn default_extract_replaces_existing_directory_symlink() {
    setup();

    let root = PathBuf::from("stdio_absolute_paths_replace_symlink");
    let archive_path = root.join("archive.pna");
    build_dir_and_file_archive(&archive_path, "dir", "dir/file.txt", "payload");

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let real_dir = root.join("real_dir");
    fs::create_dir_all(&real_dir).unwrap();
    let real_dir = fs::canonicalize(real_dir).unwrap();

    let link_dir = dist.join("dir");
    if link_dir.exists() {
        pna_fs::remove_path_all(&link_dir).unwrap();
    }
    pna_fs::symlink(&real_dir, &link_dir).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = fs::symlink_metadata(&link_dir).unwrap();
    assert!(meta.is_dir());
    assert!(!meta.file_type().is_symlink());
    assert_eq!(
        fs::read_to_string(link_dir.join("file.txt")).unwrap(),
        "payload"
    );
    assert!(!real_dir.join("file.txt").exists());
}
