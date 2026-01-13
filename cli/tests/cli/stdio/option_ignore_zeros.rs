use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, WriteOptions};
use std::io::Write;

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
