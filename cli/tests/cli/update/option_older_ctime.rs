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
/// Action: Create files with different ctimes, run `pna experimental update` with `--older-ctime`.
/// Expectation: Only files with ctime older than threshold are updated in the archive.
/// Note: This test requires filesystem support for creation time (birth time).
#[test]
fn update_with_older_ctime() {
    setup();
    let _ = fs::remove_dir_all("update_older_ctime");
    fs::create_dir_all("update_older_ctime").unwrap();

    let file_to_update = "update_older_ctime/file_to_update.txt";
    let file_to_skip = "update_older_ctime/file_to_skip.txt";

    fs::write(file_to_update, "initial content").unwrap();

    skip_unless!("birthtime", birth_time_recorded(file_to_update));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_older_ctime/archive.pna",
        "--overwrite",
        file_to_update,
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::write(file_to_update, "updated content").unwrap();
    let file_to_update_ctime = birth_time(file_to_update);

    let threshold_file = "update_older_ctime/threshold.txt";
    let threshold_ctime = create_file_born_after(
        threshold_file,
        "threshold marker",
        file_to_update_ctime + Duration::from_secs(1) - Duration::from_nanos(1),
    );

    let file_to_update_secs = file_to_update_ctime
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let threshold_secs = threshold_ctime
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(file_to_update_secs < threshold_secs);

    let file_to_skip_ctime = create_file_born_after(
        file_to_skip,
        "skip content",
        threshold_ctime + Duration::from_secs(1) - Duration::from_nanos(1),
    );
    let file_to_skip_secs = file_to_skip_ctime
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(file_to_skip_secs >= threshold_secs);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--older-ctime",
        &format!("@{threshold_secs}"),
        "-f",
        "update_older_ctime/archive.pna",
        file_to_update,
        file_to_skip,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_older_ctime/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert_eq!(seen, HashSet::from([file_to_update.to_string()]));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "update_older_ctime/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_older_ctime/out/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content =
        fs::read_to_string(format!("update_older_ctime/out/{file_to_update}")).unwrap();
    assert_eq!(
        updated_content, "updated content",
        "file_to_update should have the updated content"
    );

    assert!(
        !std::path::Path::new(&format!("update_older_ctime/out/{file_to_skip}")).exists(),
        "file_to_skip should not exist in extracted output"
    );
}
