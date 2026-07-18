use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, io::Write, path::Path};

/// Precondition: an archive contains an entry with a data-kind byte
/// unassigned by the specification, alongside a regular file entry.
/// Action: extract the archive.
/// Expectation: extraction succeeds, the unknown-kind entry is skipped
/// (no file created for it), and the regular file is extracted with its
/// contents intact.
#[test]
fn extract_skips_unknown_data_kind() {
    setup();
    let archive_path = "extract_unknown_data_kind/archive.pna";
    fs::create_dir_all(Path::new(archive_path).parent().unwrap()).unwrap();
    let mut archive = pna::Archive::write_header(fs::File::create(archive_path).unwrap()).unwrap();

    let mut unknown_builder =
        pna::OpaqueEntryBuilder::new("unknown.bin".into(), pna::DataKind::from_byte(42)).unwrap();
    unknown_builder.write_all(b"will be reinterpreted").unwrap();
    archive.add_entry(unknown_builder.build().unwrap()).unwrap();

    let mut known_builder = pna::FileEntryBuilder::new("known.txt".into()).unwrap();
    known_builder.write_all(b"regular file contents").unwrap();
    archive.add_entry(known_builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_unknown_data_kind/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_unknown_data_kind/dist",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(!Path::new("extract_unknown_data_kind/dist/unknown.bin").exists());
    assert_eq!(
        "regular file contents",
        fs::read_to_string("extract_unknown_data_kind/dist/known.txt").unwrap()
    );
}
