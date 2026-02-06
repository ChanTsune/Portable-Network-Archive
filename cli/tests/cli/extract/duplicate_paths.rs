use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryBuilder, WriteOptions};
use portable_network_archive::cli;
use std::{fs, io::Write, path::Path};

fn create_archive_with_duplicate_paths<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file("file.txt".into(), WriteOptions::builder().build()).unwrap();
            builder.write_all(b"first").unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file("file.txt".into(), WriteOptions::builder().build()).unwrap();
            builder.write_all(b"second").unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

/// Precondition: Archive contains two entries with the same path but different content.
/// Action: Extract with `--overwrite`.
/// Expectation: The file contains content from the last (second) entry.
#[test]
fn extract_duplicate_paths_uses_last_entry() {
    setup();
    create_archive_with_duplicate_paths("duplicate_paths/archive.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "duplicate_paths/archive.pna",
        "--overwrite",
        "--out-dir",
        "duplicate_paths/out",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        fs::read_to_string("duplicate_paths/out/file.txt").unwrap(),
        "second"
    );
}
