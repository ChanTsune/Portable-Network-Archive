use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, path::Path};

/// Precondition: Source tree contains an empty file and an empty directory.
/// Action: Create archive, then extract.
/// Expectation: Empty file and empty directory survive round-trip.
#[test]
fn empty_file_and_directory_round_trip() {
    setup();
    let base = "empty_entries_roundtrip";
    if Path::new(base).exists() {
        fs::remove_dir_all(base).unwrap();
    }
    fs::create_dir_all(format!("{base}/source")).unwrap();

    fs::write(format!("{base}/source/empty.txt"), b"").unwrap();
    fs::create_dir_all(format!("{base}/source/empty_dir")).unwrap();
    fs::write(format!("{base}/source/data.txt"), b"hello").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/archive.pna"),
        "--overwrite",
        "--keep-dir",
        &format!("{base}/source"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entries = HashSet::new();
    archive::for_each_entry(&format!("{base}/archive.pna"), |entry| {
        entries.insert((
            entry.header().path().to_string(),
            entry.header().data_kind(),
        ));
    })
    .unwrap();

    assert!(
        entries
            .iter()
            .any(|(p, k)| p.ends_with("empty.txt") && *k == pna::DataKind::File),
        "empty file should be in archive as File"
    );
    assert!(
        entries
            .iter()
            .any(|(p, k)| p.ends_with("empty_dir") && *k == pna::DataKind::Directory),
        "empty directory should be in archive as Directory"
    );
    assert!(
        entries
            .iter()
            .any(|(p, k)| p.ends_with("data.txt") && *k == pna::DataKind::File),
        "non-empty file should be in archive as File"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{base}/archive.pna"),
        "--overwrite",
        "--out-dir",
        &format!("{base}/dist"),
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(fs::read(format!("{base}/dist/empty.txt")).unwrap(), b"");
    assert_eq!(fs::read(format!("{base}/dist/data.txt")).unwrap(), b"hello");
    assert!(Path::new(&format!("{base}/dist/empty_dir")).is_dir());
    assert!(Path::new(&format!("{base}/dist/empty.txt")).is_file());
    assert!(Path::new(&format!("{base}/dist/data.txt")).is_file());
}
