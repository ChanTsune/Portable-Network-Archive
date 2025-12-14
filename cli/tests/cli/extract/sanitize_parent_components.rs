use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, io::Write};

/// Precondition: An archive contains a file entry whose stored name includes `..` (e.g. `../escape.txt`).
/// Action: Run `pna extract` with `--out-dir` pointing to a directory.
/// Expectation: The extracted file is placed under the out-dir after sanitization and does not escape it.
#[test]
fn extract_command_sanitizes_parent_components_in_entry_names() {
    setup();

    fs::create_dir_all("extract_sanitize_parent_components/out").unwrap();

    let archive_file = fs::File::create("extract_sanitize_parent_components/archive.pna").unwrap();
    let mut archive = pna::Archive::write_header(archive_file).unwrap();

    let raw_name = pna::EntryName::from_utf8_preserve_root("../escape.txt");
    let mut builder = pna::EntryBuilder::new_file(raw_name, pna::WriteOptions::store()).unwrap();
    builder.write_all(b"payload").unwrap();
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_sanitize_parent_components/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_sanitize_parent_components/out",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(
        !fs::exists("extract_sanitize_parent_components/escape.txt").unwrap(),
        "entry escaped out-dir"
    );
    assert_eq!(
        fs::read("extract_sanitize_parent_components/out/escape.txt").unwrap(),
        b"payload"
    );
}
