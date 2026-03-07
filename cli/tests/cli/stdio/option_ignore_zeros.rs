use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, ReadOptions, WriteOptions};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn build_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = Archive::write_header(Vec::new()).unwrap();
    for (name, content) in entries {
        let mut builder = EntryBuilder::new_file((*name).into(), WriteOptions::store()).unwrap();
        builder.write_all(content).unwrap();
        archive.add_entry(builder.build().unwrap()).unwrap();
    }
    archive.finalize().unwrap()
}

fn build_concatenated_archives() -> Vec<u8> {
    let mut archives = build_archive(&[("a.txt", b"first" as &[u8])]);
    archives.extend(build_archive(&[("b.txt", b"second" as &[u8])]));
    archives
}

fn read_archive_entries(path: impl AsRef<Path>) -> Vec<(String, String)> {
    let mut archive = Archive::read_header(fs::File::open(path).unwrap()).unwrap();
    archive
        .entries()
        .extract_solid_entries(None)
        .map(|entry| {
            let entry = entry.unwrap();
            let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
            let mut content = String::new();
            reader.read_to_string(&mut content).unwrap();
            (entry.name().to_string(), content)
        })
        .collect()
}

#[test]
fn stdio_list_ignore_zeros_controls_concatenated_archive_handling() {
    setup();
    let archive_data = build_concatenated_archives();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data.clone())
        .args(["experimental", "stdio", "--list"])
        .assert()
        .success()
        .stdout("a.txt\n")
        .stderr("");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args(["experimental", "stdio", "--list", "--ignore-zeros"])
        .assert()
        .success()
        .stdout("a.txt\nb.txt\n")
        .stderr("");
}

#[test]
fn stdio_list_ignore_zeros_with_fast_read_continues_into_next_archive() {
    setup();
    let archive_data = build_concatenated_archives();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args([
            "experimental",
            "stdio",
            "--list",
            "--ignore-zeros",
            "--fast-read",
            "b.txt",
        ])
        .assert()
        .success()
        .stdout("b.txt\n")
        .stderr("");
}

#[test]
fn stdio_extract_ignore_zeros_controls_concatenated_archive_handling() {
    setup();
    let archive_data = build_concatenated_archives();
    let out_without = PathBuf::from("stdio_extract_ignore_zeros_without_flag/out");
    let out_with = PathBuf::from("stdio_extract_ignore_zeros_with_flag/out");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data.clone())
        .args([
            "experimental",
            "stdio",
            "--extract",
            "--out-dir",
            out_without.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr("");

    assert_eq!(
        "first",
        fs::read_to_string(out_without.join("a.txt")).unwrap()
    );
    assert!(!out_without.join("b.txt").exists());

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args([
            "experimental",
            "stdio",
            "--extract",
            "--ignore-zeros",
            "--out-dir",
            out_with.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr("");

    assert_eq!("first", fs::read_to_string(out_with.join("a.txt")).unwrap());
    assert_eq!(
        "second",
        fs::read_to_string(out_with.join("b.txt")).unwrap()
    );
}

#[test]
fn stdio_extract_ignore_zeros_with_fast_read_continues_into_next_archive() {
    setup();
    let archive_data = build_concatenated_archives();
    let out_dir = PathBuf::from("stdio_extract_ignore_zeros_fast_read/out");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args([
            "experimental",
            "stdio",
            "--extract",
            "--ignore-zeros",
            "--fast-read",
            "--out-dir",
            out_dir.to_str().unwrap(),
            "b.txt",
        ])
        .assert()
        .success()
        .stderr("");

    assert!(!out_dir.join("a.txt").exists());
    assert_eq!("second", fs::read_to_string(out_dir.join("b.txt")).unwrap());
}

#[test]
fn stdio_update_ignore_zeros_controls_concatenated_archive_handling() {
    setup();
    let base = PathBuf::from("stdio_update_ignore_zeros");
    let in_dir = base.join("in");
    let archive_without = base.join("without_ignore.pna");
    let archive_with = base.join("with_ignore.pna");

    fs::create_dir_all(&in_dir).unwrap();
    fs::write(in_dir.join("c.txt"), "third").unwrap();
    fs::write(&archive_without, build_concatenated_archives()).unwrap();
    fs::write(&archive_with, build_concatenated_archives()).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--update",
        "--file",
        archive_without.to_str().unwrap(),
        "--cd",
        in_dir.to_str().unwrap(),
        "c.txt",
    ])
    .assert()
    .success()
    .stderr("");

    assert_eq!(
        read_archive_entries(&archive_without),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--update",
        "--ignore-zeros",
        "--file",
        archive_with.to_str().unwrap(),
        "--cd",
        in_dir.to_str().unwrap(),
        "c.txt",
    ])
    .assert()
    .success()
    .stderr("");

    assert_eq!(
        read_archive_entries(&archive_with),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("b.txt".to_string(), "second".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );
}
