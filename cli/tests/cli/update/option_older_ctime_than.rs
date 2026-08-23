use crate::utils::{
    archive, setup,
    time::{birth_time, birth_time_recorded, create_file_born_after},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: An archive exists with files to update, and the source tree contains a reference file and files with varying creation times.
/// Action: Run `pna experimental update` with `--older-ctime-than` pointing to the reference file.
/// Expectation: Only files whose creation time is older than the reference file are processed in the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn update_with_older_ctime_than() {
    setup();
    let base_dir = "update_older_ctime_than";
    let archive_path = format!("{base_dir}/test.pna");
    let file_to_update = format!("{base_dir}/file_to_update.txt");
    let file_to_skip = format!("{base_dir}/file_to_skip.txt");
    let reference_file = format!("{base_dir}/reference.txt");

    fs::create_dir_all(base_dir).unwrap();
    fs::write(&file_to_update, "initial content").unwrap();

    skip_unless!("birthtime", birth_time_recorded(&file_to_update));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &archive_path,
        "--overwrite",
        &file_to_update,
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::write(&file_to_update, "updated content").unwrap();

    let reference_ctime = create_file_born_after(
        &reference_file,
        "reference marker",
        birth_time(&file_to_update),
    );
    create_file_born_after(&file_to_skip, "skip content", reference_ctime);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--overwrite",
        "--file",
        &archive_path,
        &file_to_update,
        &file_to_skip,
        "--unstable",
        "--older-ctime-than",
        &reference_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(&archive_path, |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(seen, HashSet::from([file_to_update.clone()]));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--file",
        &archive_path,
        "--out-dir",
        &format!("{base_dir}/out"),
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content = fs::read_to_string(format!("{base_dir}/out/{file_to_update}")).unwrap();
    assert_eq!(updated_content, "updated content");
    assert!(
        fs::metadata(format!("{base_dir}/out/{file_to_skip}")).is_err(),
        "skip file should not have been extracted/added"
    );
}
