use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, io, thread, time};

/// Test archive updating with `--newer-ctime-than` option.
///
/// This test verifies that the `update` command correctly adds and updates files
/// based on their creation time (`ctime`) when using the `--newer-ctime-than` flag.
/// It creates a reference file to establish a time benchmark, then updates one
/// existing archive entry and adds one new file, both of which are newer than
/// the reference.
#[test]
fn update_with_newer_ctime_than() -> io::Result<()> {
    setup();
    let reference_file = "reference.txt";
    let file_to_update = "file_to_update.txt";
    let file_to_add = "file_to_add.txt";

    // 1. Create the initial file and archive it.
    fs::write(file_to_update, "initial content")?;
    cli::Cli::try_parse_from(["pna", "c", "test.pna", file_to_update, "--overwrite"])
        .unwrap()
        .execute()
        .unwrap();

    // 2. Create a reference file to set a timestamp benchmark.
    thread::sleep(time::Duration::from_secs(1));
    fs::write(reference_file, "time reference")?;

    // 3. Wait, then update the existing file and create the new file to ensure they
    //    have a `ctime` newer than the reference file.
    thread::sleep(time::Duration::from_secs(1));
    fs::write(file_to_update, "updated content")?;
    fs::write(file_to_add, "new file content")?;

    // 4. Run the update command, targeting the files to be updated/added,
    //    filtered by the ctime of the reference file.
    cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "update",
        "--file",
        "test.pna",
        file_to_update,
        file_to_add,
        "--unstable",
        "--newer-ctime-than",
        reference_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // 5. Extract the archive to a new directory for verification.
    cli::Cli::try_parse_from([
        "pna",
        "x",
        "--file",
        "test.pna",
        "--out-dir",
        "out",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // 6. Verify that only the newly added and the updated files are present.
    let entries: Vec<_> = fs::read_dir("out")?
        .map(|res| res.map(|e| e.file_name()))
        .collect::<Result<Vec<_>, io::Error>>()?;

    assert_eq!(
        entries.len(),
        2,
        "Expected 2 files in the archive, but found {}",
        entries.len()
    );

    let mut entry_names: Vec<_> = entries.into_iter().map(|e| e.into_string().unwrap()).collect();
    entry_names.sort();

    assert_eq!(
        entry_names[0], file_to_add,
        "Expected the new file to be in the archive"
    );
    assert_eq!(
        entry_names[1], file_to_update,
        "Expected the updated file to be in the archive"
    );

    // 7. Verify the content of the updated file.
    let updated_content = fs::read_to_string(format!("out/{}", file_to_update))?;
    assert_eq!(
        updated_content, "updated content",
        "The updated file did not contain the correct content"
    );

    Ok(())
}
