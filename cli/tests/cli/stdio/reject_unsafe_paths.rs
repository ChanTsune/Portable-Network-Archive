use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, EntryName, WriteOptions};
use std::fs;
use std::io::Write;

#[test]
fn stdio_extract_rejects_parent_components_in_entry_names() {
    setup();

    let _ = fs::remove_dir_all("stdio_reject_unsafe_paths");
    fs::create_dir_all("stdio_reject_unsafe_paths/out").unwrap();

    let archive_file = fs::File::create("stdio_reject_unsafe_paths/archive.pna").unwrap();
    let mut archive = Archive::write_header(archive_file).unwrap();
    archive
        .add_entry({
            let raw_name = EntryName::from_utf8_preserve_root("../escape.txt");
            let mut builder = EntryBuilder::new_file(raw_name, WriteOptions::store()).unwrap();
            builder.write_all(b"payload").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    archive.finalize().unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--file",
            "stdio_reject_unsafe_paths/archive.pna",
            "--out-dir",
            "stdio_reject_unsafe_paths/out",
        ])
        .assert()
        .failure();

    assert!(
        !fs::exists("stdio_reject_unsafe_paths/out/escape.txt").unwrap(),
        "unsafe entry must not be extracted"
    );
    assert!(
        !fs::exists("stdio_reject_unsafe_paths/escape.txt").unwrap(),
        "unsafe entry must not escape the out-dir"
    );
}
