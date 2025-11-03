use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, io, thread, time};

/// Test archive creation with `--newer-mtime-than` option.
///
/// This test creates two files with different modification times, then creates an
/// archive with the `--newer-mtime-than` option and verifies that only the
/// newer file is included in the archive.
#[test]
fn create_with_newer_mtime_than() -> io::Result<()> {
    setup();
    let older_file = "older.txt";
    let newer_file = "newer.txt";

    // Create the older file
    fs::write(older_file, "older file content")?;

    // Wait to ensure the next file has a newer mtime
    thread::sleep(time::Duration::from_secs(1));

    // Create the newer file
    fs::write(newer_file, "newer file content")?;

    // Create an archive with the `--newer-mtime-than` option
    cli::Cli::try_parse_from([
        "pna",
        "c",
        "test.pna",
        "--overwrite",
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

    // Verify that only the newer file is in the extracted directory
    let mut entries = fs::read_dir("out")?;
    assert_eq!(entries.next().unwrap()?.file_name(), newer_file);
    assert!(entries.next().is_none());

    Ok(())
}
