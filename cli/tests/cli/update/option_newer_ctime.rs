use crate::utils::{
    archive, setup,
    time::{birth_time, birth_time_recorded, create_file_born_after},
};
use clap::Parser;
use portable_network_archive::cli;
use std::{
    collections::HashSet,
    fs,
    time::{Duration, SystemTime},
};

/// Precondition: An archive contains a file.
/// Action: Recreate files with different ctimes, run `pna experimental update` with `--newer-ctime`.
/// Expectation: Only files with ctime newer than threshold are updated in the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn update_with_newer_ctime() {
    setup();
    let _ = fs::remove_dir_all("update_newer_ctime");
    fs::create_dir_all("update_newer_ctime").unwrap();

    let file_to_keep = "update_newer_ctime/file_to_keep.txt";
    let file_to_update = "update_newer_ctime/file_to_update.txt";

    fs::write(file_to_keep, "original content").unwrap();

    skip_unless!("birthtime", birth_time_recorded(file_to_keep));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_newer_ctime/archive.pna",
        "--overwrite",
        file_to_keep,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let threshold_file = "update_newer_ctime/threshold.txt";
    let threshold_ctime =
        create_file_born_after(threshold_file, "threshold marker", birth_time(file_to_keep));

    // `--newer-ctime @secs` truncates to whole seconds, so round the boundary
    // up to the next second: threshold_file's own sub-second ctime then falls
    // strictly before it (excluded) while file_to_update, created after the
    // boundary, falls strictly after it (included).
    let boundary_secs = threshold_ctime
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;
    create_file_born_after(
        file_to_update,
        "new content",
        SystemTime::UNIX_EPOCH + Duration::from_secs(boundary_secs),
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--overwrite",
        "--newer-ctime",
        &format!("@{boundary_secs}"),
        "-f",
        "update_newer_ctime/archive.pna",
        file_to_keep,
        file_to_update,
        threshold_file,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_newer_ctime/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(
        seen,
        HashSet::from([file_to_keep.to_string(), file_to_update.to_string()])
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "update_newer_ctime/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_newer_ctime/out/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content =
        fs::read_to_string(format!("update_newer_ctime/out/{file_to_update}")).unwrap();
    assert_eq!(
        updated_content, "new content",
        "file_to_update should have the new content"
    );
}
