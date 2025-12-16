use crate::utils::setup;
use clap::Parser;
use pna::{Archive, Duration, EntryBuilder, WriteOptions};
use portable_network_archive::cli;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn init_file_archive<P: AsRef<Path>>(path: P, modified: Option<Duration>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let file = fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    let mut builder =
        EntryBuilder::new_file("file.txt".into(), WriteOptions::builder().build()).unwrap();
    if let Some(mtime) = modified {
        builder.modified(mtime);
    }
    builder.write_all(b"from archive").unwrap();
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
}

/// Precondition: Archive `file.txt` is present, and an existing filesystem copy already exists.
/// Action: Extract via `pna extract` with `--keep-old-files`, directing output to the same location.
/// Expectation: The existing file is preserved and the archive payload is skipped.
#[test]
fn keep_older_preserves_existing_files() {
    setup();
    init_file_archive("keep_strategy/keep_older/archive.pna", None);

    let out_dir = PathBuf::from("keep_strategy/keep_older/out");
    fs::create_dir_all(&out_dir).unwrap();
    let target = out_dir.join("file.txt");
    fs::write(&target, b"existing").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "keep_strategy/keep_older/out",
        "--keep-old-files",
        "-f",
        "keep_strategy/keep_older/archive.pna",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"existing");
}
