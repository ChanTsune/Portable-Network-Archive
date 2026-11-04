use crate::utils::{
    archive, setup,
    time::{birth_time, birth_time_recorded, create_file_born_after},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: An archive exists with files to update, and the source tree contains a reference file and files with varying creation times.
/// Action: Run `pna experimental update` with `--newer-ctime-than` pointing to the reference file.
/// Expectation: Only files whose creation time is newer than the reference file are updated or added to the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn update_with_newer_ctime_than() {
    setup();
    let reference_file = "update_newer_ctime_than/reference.txt";
    let file_to_update = "update_newer_ctime_than/file_to_update.txt";
    let file_to_add = "update_newer_ctime_than/file_to_add.txt";

    fs::create_dir_all("update_newer_ctime_than").unwrap();
    fs::write(file_to_update, "initial content").unwrap();

    skip_unless!("birthtime", birth_time_recorded(file_to_update));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_newer_ctime_than/test.pna",
        "--overwrite",
        file_to_update,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let reference_ctime =
        create_file_born_after(reference_file, "time reference", birth_time(file_to_update));
    create_file_born_after(file_to_update, "updated content", reference_ctime);
    create_file_born_after(file_to_add, "new file content", reference_ctime);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--file",
        "update_newer_ctime_than/test.pna",
        file_to_update,
        file_to_add,
        reference_file,
        "--unstable",
        "--newer-ctime-than",
        reference_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_newer_ctime_than/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(
        seen,
        HashSet::from([file_to_update.to_string(), file_to_add.to_string()])
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--file",
        "update_newer_ctime_than/test.pna",
        "--out-dir",
        "update_newer_ctime_than/out",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content = fs::read_to_string(
        "update_newer_ctime_than/out/update_newer_ctime_than/file_to_update.txt",
    )
    .unwrap();
    assert_eq!(
        updated_content, "updated content",
        "The updated file did not contain the correct content"
    );
}
