use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, WriteOptions};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn build_duplicate_archive(path: impl AsRef<Path>) -> Vec<u8> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    for (name, content) in [
        ("a.txt", b"first-a" as &[u8]),
        ("a.txt", b"second-a"),
        ("b.txt", b"first-b"),
        ("b.txt", b"second-b"),
    ] {
        let mut builder = EntryBuilder::new_file(name.into(), WriteOptions::store()).unwrap();
        builder.write_all(content).unwrap();
        archive.add_entry(builder.build().unwrap()).unwrap();
    }

    archive.finalize().unwrap();
    fs::read(path).unwrap()
}

#[test]
fn stdio_list_fast_read_first_match_only() {
    setup();
    let archive_data = build_duplicate_archive("stdio_fast_read_list/archive.pna");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args(["experimental", "stdio", "--list", "-q", "a.txt", "b.txt"])
        .assert()
        .success()
        .stdout("a.txt\nb.txt\n");
}

#[test]
fn stdio_list_fast_read_no_operands() {
    setup();
    let archive_data = build_duplicate_archive("stdio_fast_read_list_no_operands/archive.pna");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args(["experimental", "stdio", "--list", "--fast-read"])
        .assert()
        .success()
        .stdout("a.txt\na.txt\nb.txt\nb.txt\n");
}

#[test]
fn stdio_extract_fast_read_keeps_first_entry() {
    setup();
    let archive_data = build_duplicate_archive("stdio_fast_read_extract/archive.pna");
    let out_dir = PathBuf::from("stdio_fast_read_extract/out_fast");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args([
            "experimental",
            "stdio",
            "--extract",
            "--fast-read",
            "--overwrite",
            "--out-dir",
            out_dir.to_str().unwrap(),
            "a.txt",
            "b.txt",
        ])
        .assert()
        .success();

    assert_eq!(
        "first-a",
        fs::read_to_string(out_dir.join("a.txt")).unwrap()
    );
    assert_eq!(
        "first-b",
        fs::read_to_string(out_dir.join("b.txt")).unwrap()
    );
}

#[test]
fn stdio_extract_default_last_entry_wins() {
    setup();
    let archive_data = build_duplicate_archive("stdio_fast_read_extract_default/archive.pna");
    let out_dir = PathBuf::from("stdio_fast_read_extract_default/out_default");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args([
            "experimental",
            "stdio",
            "--extract",
            "--overwrite",
            "--out-dir",
            out_dir.to_str().unwrap(),
            "a.txt",
            "b.txt",
        ])
        .assert()
        .success();

    assert_eq!(
        "second-a",
        fs::read_to_string(out_dir.join("a.txt")).unwrap()
    );
    assert_eq!(
        "second-b",
        fs::read_to_string(out_dir.join("b.txt")).unwrap()
    );
}
