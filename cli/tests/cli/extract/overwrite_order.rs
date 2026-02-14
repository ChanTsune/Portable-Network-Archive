use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, io::Write};

/// Precondition: Archive contains multiple entries with the same path but different contents.
/// Action: Extract with --overwrite enabled.
/// Expectation: The last entry in archive order determines the final file content.
#[test]
fn extract_duplicate_entries_last_wins() {
    setup();

    let archive_path = "overwrite_order/last_wins/archive.pna";
    fs::create_dir_all("overwrite_order/last_wins").unwrap();

    let file = fs::File::create(archive_path).unwrap();
    let mut archive = pna::Archive::write_header(file).unwrap();

    for content in [b"aaa", b"bbb", b"ccc"] {
        let mut builder =
            pna::EntryBuilder::new_file("file.txt".into(), pna::WriteOptions::store()).unwrap();
        builder.write_all(content).unwrap();
        archive.add_entry(builder.build().unwrap()).unwrap();
    }

    archive.finalize().unwrap();

    let out_dir = "overwrite_order/last_wins/out";
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        out_dir,
        "-f",
        archive_path,
        "--overwrite",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        fs::read_to_string(format!("{out_dir}/file.txt")).unwrap(),
        "ccc"
    );
}
