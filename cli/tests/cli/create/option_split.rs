use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, io::prelude::*, path::Path};

/// Precondition: The input files fit within a single part of the requested split size.
/// Action: Run `pna create` with `--split`.
/// Expectation: The archive is written to the requested path with no part-numbered
/// file left behind, and its entries round-trip.
#[test]
fn create_with_split_fitting_in_single_part() {
    setup();
    if Path::new("create_split_single_part").exists() {
        fs::remove_dir_all("create_split_single_part").unwrap();
    }
    fs::create_dir_all("create_split_single_part/in/").unwrap();
    fs::write("create_split_single_part/in/first.txt", b"first").unwrap();
    fs::write("create_split_single_part/in/second.txt", b"second").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_split_single_part/archive.pna",
        "--overwrite",
        "--unstable",
        "--split",
        "100kb",
        "create_split_single_part/in/first.txt",
        "create_split_single_part/in/second.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(
        Path::new("create_split_single_part/archive.pna").exists(),
        "a single-part split should be written to the requested path"
    );
    assert!(
        !Path::new("create_split_single_part/archive.part1.pna").exists(),
        "a single-part split should not leave a part-numbered file behind"
    );

    let mut seen = HashSet::new();
    archive::for_each_entry("create_split_single_part/archive.pna", |entry| {
        let path = entry.header().path().to_string();
        let mut contents = Vec::new();
        entry
            .reader(pna::ReadOptions::with_password::<&[u8]>(None))
            .unwrap()
            .read_to_end(&mut contents)
            .unwrap();
        seen.insert((path, contents));
    })
    .unwrap();

    assert_eq!(
        seen,
        HashSet::from([
            (
                "create_split_single_part/in/first.txt".to_string(),
                b"first".to_vec()
            ),
            (
                "create_split_single_part/in/second.txt".to_string(),
                b"second".to_vec()
            ),
        ])
    );
}
