use crate::utils::{archive::for_each_entry, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs,
    io::prelude::*,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DURATION_24_HOURS: Duration = Duration::from_secs(24 * 60 * 60);

#[test]
fn archive_create_with_ctime() {
    setup();
    let test_archive_dir = "archive_create_with_ctime";
    let input_dir = format!("{}/in/", test_archive_dir);
    let output_archive = format!("{}/create_with_ctime.pna", test_archive_dir);

    TestResources::extract_in("raw/", &input_dir).unwrap();

    // Modify a file's mtime to ensure we're not accidentally picking it up as ctime
    // Note: Modifying mtime might also update ctime on some systems, but the explicit
    // --ctime flag should override this.
    let file_to_modify_path = format!("{}raw/text.txt", input_dir);
    let original_meta = fs::metadata(&file_to_modify_path).unwrap();
    let original_mtime = original_meta.modified().unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open(&file_to_modify_path)
        .unwrap();
    file.write_all(b"updated for ctime test!").unwrap();
    drop(file); // Close the file to ensure metadata changes are flushed

    // Attempt to set mtime to something clearly different from the target ctime
    // This is a best-effort to isolate ctime setting.
    let new_mtime = original_mtime.checked_sub(DURATION_24_HOURS).unwrap_or(UNIX_EPOCH + DURATION_24_HOURS);
    filetime::set_file_mtime(&file_to_modify_path, filetime::FileTime::from_system_time(new_mtime)).unwrap();


    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &output_archive,
        "--overwrite",
        &input_dir,
        "--keep-timestamp",
        "--ctime",
        "2023-01-01T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let expected_ctime_duration = Duration::from_secs(1672531200); // 2023-01-01T00:00:00Z
    let mut checked = false;
    for_each_entry(&output_archive, |entry| {
        assert_eq!(entry.metadata().created(), Some(expected_ctime_duration));
        checked = true;
    })
    .unwrap();
    assert!(checked, "No entries were checked in the archive_create_with_ctime test");
}

#[test]
fn archive_create_with_clamp_ctime() {
    setup();
    let test_archive_dir = "archive_create_with_clamp_ctime";
    let input_dir = format!("{}/in/", test_archive_dir);
    let output_archive = format!("{}/create_with_clamp_ctime.pna", test_archive_dir);

    TestResources::extract_in("raw/", &input_dir).unwrap();

    // For one file, set its mtime significantly in the past, which might also set its ctime in the past.
    // This file ("old_file.txt") should retain its old ctime if it's older than the clamp ctime.
    let old_file_path = format!("{}raw/text.txt", input_dir); // Using text.txt as the "old" file for simplicity
    fs::write(&old_file_path, "This file is made to be old").unwrap();
    let ancient_time = UNIX_EPOCH + Duration::from_secs(1000000000); // Somewhere in 2001
    filetime::set_file_times(
        &old_file_path,
        filetime::FileTime::from_system_time(ancient_time), // atime
        filetime::FileTime::from_system_time(ancient_time), // mtime
    ).unwrap();
    // We expect ctime to be `ancient_time` or slightly after if system updated it during mtime set.
    // On some systems, ctime might be updated to 'now' when mtime is set.
    // The test relies on the OS behavior of ctime. If ctime becomes 'now',
    // then clamp_ctime will ensure it's clamped to "2023-01-01T00:00:00Z".
    // If the OS preserves an older ctime (older than 2023-01-01), that older ctime should be kept.


    // For another file, ensure its ctime is likely newer than the clamp ctime.
    // (e.g. by creating it or modifying it now)
    let new_file_path = format!("{}raw/newly_touched_file.txt", input_dir);
    fs::write(&new_file_path, "This file is new or recently touched.").unwrap();
    // Its ctime will be 'now'.

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &output_archive,
        "--overwrite",
        &input_dir,
        "--keep-timestamp",
        "--ctime",
        "2023-01-01T00:00:00Z", // Clamp date: January 1, 2023
        "--clamp-ctime",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let clamp_date_secs = 1672531200; // 2023-01-01T00:00:00Z
    let mut checked_at_least_one_entry = false;
    for_each_entry(&output_archive, |entry| {
        let entry_created_duration = entry.metadata().created().expect("Entry should have ctime");
        let entry_created_secs = entry_created_duration.as_secs();

        let entry_path_str = entry.header().path().as_str();

        if entry_path_str.ends_with("newly_touched_file.txt") {
            // This file's natural ctime is 'now' (approx > 2023).
            // So, it should be clamped to 2023-01-01T00:00:00Z.
            assert_eq!(entry_created_secs, clamp_date_secs,
                       "Newly touched file '{}' ctime should be clamped to {}", entry_path_str, clamp_date_secs);
        } else {
            // For other files (like the one we tried to make "old", or others from "raw/"),
            // their ctime should be less than or equal to the clamp_date_secs.
            // If their original ctime was older than clamp_date_secs, it's preserved.
            // If their original ctime was newer (e.g. due to OS behavior on mtime set), it's clamped.
             assert!(entry_created_secs <= clamp_date_secs,
                       "Entry '{}' ctime {} should be less than or equal to clamp ctime {}",
                       entry_path_str, entry_created_secs, clamp_date_secs);
        }
        checked_at_least_one_entry = true;
    })
    .unwrap();
    assert!(checked_at_least_one_entry, "No entries were checked in the archive_create_with_clamp_ctime test");
}
