use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, io, thread, time};

/// Test appending to an archive with `--newer-mtime-than` option.
///
/// This test creates two files with different modification times, creates an
/// archive with the older file, then appends to the archive with the
/// `--newer-mtime-than` option and verifies that only the newer file is added.
#[test]
fn append_with_newer_mtime_than() -> io::Result<()> {
    setup();
    let older_file = "older.txt";
    let newer_file = "newer.txt";

    // Create the older file
    fs::write(older_file, "older file content")?;

    // Create an archive with the older file
    cli::Cli::try_parse_from(["pna", "c", "test.pna", older_file, "--overwrite"])
        .unwrap()
        .execute()
        .unwrap();

    // Wait to ensure the next file has a newer mtime
    thread::sleep(time::Duration::from_secs(1));

    // Create the newer file
    fs::write(newer_file, "newer file content")?;

    // Append to the archive with the `--newer-mtime-than` option
    cli::Cli::try_parse_from([
        "pna",
        "a",
        "test.pna",
        "--unstable",
        "--newer-mtime-than",
        older_file,
        newer_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract the archive to a new directory
    cli::Cli::try_parse_from(["pna", "x", "test.pna", "--out-dir", "out"])
        .unwrap()
        .execute()
        .unwrap();

    // Verify that both files are in the extracted directory
    let mut entries: Vec<_> = fs::read_dir("out")?
        .map(|res| res.map(|e| e.file_name()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0], older_file);
    assert_eq!(entries[1], newer_file);

    Ok(())
}
