use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, EntryName, WriteOptions};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn build_archive(path: &PathBuf, entries: &[(&str, &str)]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    for (name, contents) in entries {
        let mut builder = EntryBuilder::new_file(
            EntryName::from_utf8_preserve_root(*name),
            WriteOptions::builder().build(),
        )
        .unwrap();
        builder.write_all(contents.as_bytes()).unwrap();
        writer.add_entry(builder.build().unwrap()).unwrap();
    }
    writer.finalize().unwrap();
}

#[test]
fn create_default_strips_leading_slash() {
    setup();
    let root = PathBuf::from("stdio_abs_create");
    fs::create_dir_all(&root).unwrap();
    let abs_file_path = root.join("input.txt");
    fs::write(&abs_file_path, b"payload").unwrap();
    let abs_file = fs::canonicalize(&abs_file_path).unwrap();
    let archive_path = root.join("archive.pna");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--file",
            archive_path.to_str().unwrap(),
            "--overwrite",
            abs_file.to_str().unwrap(),
            "--unstable",
        ])
        .assert()
        .success();

    let mut names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(names.len(), 1);
    assert!(
        !names[0].starts_with('/'),
        "default create should strip leading slash: {:?}",
        names[0]
    );
}

#[test]
fn create_with_absolute_paths_preserves_leading_slash() {
    setup();
    let root = PathBuf::from("stdio_abs_create_p");
    fs::create_dir_all(&root).unwrap();
    let abs_file_path = root.join("input.txt");
    fs::write(&abs_file_path, b"payload").unwrap();
    let abs_file = fs::canonicalize(&abs_file_path).unwrap();
    let archive_path = root.join("archive.pna");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--absolute-paths",
            "--file",
            archive_path.to_str().unwrap(),
            "--overwrite",
            abs_file.to_str().unwrap(),
            "--unstable",
        ])
        .assert()
        .success();

    let mut names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(names.len(), 1);
    assert!(
        names[0].starts_with('/'),
        "absolute-paths should preserve leading slash: {:?}",
        names[0]
    );
}

#[test]
fn extract_default_sanitizes_parent_components() {
    setup();
    let root = PathBuf::from("stdio_abs_extract_default");
    let archive_path = root.join("archive.pna");
    build_archive(&archive_path, &[("../escape.txt", "x"), ("safe.txt", "y")]);

    let out_dir = root.join("out");
    fs::create_dir_all(&out_dir).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
            "--overwrite",
            "--unstable",
        ])
        .assert()
        .success();

    assert!(out_dir.join("safe.txt").is_file());
    assert!(
        out_dir.join("escape.txt").is_file(),
        "parent components should be stripped by default"
    );
    assert!(
        !root.join("escape.txt").exists(),
        "default extraction must not traverse out of out_dir"
    );
}

#[test]
fn extract_with_absolute_paths_preserves_parent_components() {
    setup();
    let root = PathBuf::from("stdio_abs_extract_p");
    let _ = fs::remove_dir_all(&root);
    let archive_path = root.join("archive.pna");
    build_archive(&archive_path, &[("../escape.txt", "x"), ("safe.txt", "y")]);

    let out_dir = root.join("out");
    fs::create_dir_all(&out_dir).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--absolute-paths",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
            "--overwrite",
            "--unstable",
        ])
        .assert()
        .success();

    assert!(out_dir.join("safe.txt").is_file());
    assert!(
        !out_dir.join("escape.txt").exists(),
        "absolute-paths should preserve parent traversal, so file leaves out_dir"
    );
    assert!(
        root.join("escape.txt").is_file(),
        "parent traversal should resolve relative to out_dir parent"
    );
}
