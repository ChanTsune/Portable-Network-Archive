use crate::utils::{archive::for_each_entry, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command, EntryName};
use std::{
    collections::HashMap,
    fs,
    io::prelude::*,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DURATION_72_HOURS: Duration = Duration::from_secs(72 * 60 * 60);

// Helper function to get original ctimes from an archive
fn get_original_ctimes_from_archive(archive_path: &str) -> HashMap<EntryName, Duration> {
    let mut ctimes = HashMap::new();
    for_each_entry(archive_path, |entry| {
        if let Some(created) = entry.metadata().created() {
            ctimes.insert(entry.header().path().clone(), created);
        }
    })
    .unwrap();
    ctimes
}

#[test]
fn archive_append_with_ctime() {
    setup();
    let test_dir = "archive_append_with_ctime";
    let initial_archive_path = format!("{}/initial_append.pna", test_dir);
    let initial_files_dir = format!("{}/initial_files/", test_dir); // Files for the initial archive
    let files_to_append_root_dir = format!("{}/files_to_append/", test_dir); // Files to be appended

    let file_to_append_rel_path = "newly_added_file.txt";
    let file_to_append_abs_path = PathBuf::from(&files_to_append_root_dir).join(file_to_append_rel_path);

    // Setup initial files and files to append
    TestResources::extract_in("raw/", &initial_files_dir).unwrap();
    fs::create_dir_all(file_to_append_abs_path.parent().unwrap()).unwrap();
    fs::write(&file_to_append_abs_path, "This is a new file for append ctime test.").unwrap();

    // Set mtime of the file to be appended to something old to ensure ctime logic is what's tested
    // On some OS, writing to a file updates mtime and ctime. We want to control mtime here.
    let old_mtime = SystemTime::now()
        .checked_sub(DURATION_72_HOURS)
        .unwrap_or(UNIX_EPOCH);
    filetime::set_file_mtime(
        &file_to_append_abs_path,
        filetime::FileTime::from_system_time(old_mtime),
    )
    .unwrap();

    // Create an initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &initial_archive_path,
        "--overwrite",
        &initial_files_dir, // Add all files from this dir
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let original_ctimes = get_original_ctimes_from_archive(&initial_archive_path);
    assert!(!original_ctimes.is_empty(), "Original archive should have entries with ctimes");

    // Append to the archive with a specific ctime
    let append_ctime_str = "2023-03-01T00:00:00Z";
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        &initial_archive_path,
        file_to_append_rel_path, // Append this new file, relative to -C dir
        "--keep-timestamp",
        "--ctime",
        append_ctime_str,
        "-C", // Change directory to where the source files for append are
        &files_to_append_root_dir,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let expected_append_ctime_duration = Duration::from_secs(1677628800); // 2023-03-01T00:00:00Z
    let mut appended_entry_checked = false;
    let mut original_entries_checked_count = 0;

    for_each_entry(&initial_archive_path, |entry| {
        let entry_path_str = entry.header().path().as_str();
        if entry_path_str == file_to_append_rel_path {
            assert_eq!(
                entry.metadata().created(),
                Some(expected_append_ctime_duration),
                "Appended entry '{}' ctime mismatch",
                entry_path_str
            );
            appended_entry_checked = true;
        } else {
            // Check that other entries retained their original ctime
            if let Some(original_ctime) = original_ctimes.get(entry.header().path()) {
                assert_eq!(
                    entry.metadata().created().as_ref(),
                    Some(original_ctime),
                    "Original entry '{}' ctime should not have changed from {:?}",
                    entry_path_str,
                    original_ctime
                );
                original_entries_checked_count += 1;
            } else {
                // This case should ideally not happen if all original entries had ctime
                panic!("Original ctime not found for entry: {}", entry_path_str);
            }
        }
    })
    .unwrap();
    assert!(
        appended_entry_checked,
        "The appended entry '{}' was not checked.",
        file_to_append_rel_path
    );
    assert_eq!(
        original_entries_checked_count,
        original_ctimes.len(),
        "Number of checked original entries does not match the initial count."
    );
}

#[test]
fn archive_append_with_clamp_ctime() {
    setup();
    let test_dir = "archive_append_with_clamp_ctime";
    let initial_archive_path = format!("{}/initial_append_clamp.pna", test_dir);
    let initial_files_dir = format!("{}/initial_files_clamp/", test_dir);
    let files_to_append_root_dir = format!("{}/files_to_append_clamp/", test_dir);

    let file_to_append_rel_path = "newly_added_clamp_file.txt";
    let file_to_append_abs_path =
        PathBuf::from(&files_to_append_root_dir).join(file_to_append_rel_path);
    
    // File whose actual ctime will be older than the clamp date
    let old_file_to_append_rel_path = "old_appended_file.txt";
    let old_file_to_append_abs_path = PathBuf::from(&files_to_append_root_dir).join(old_file_to_append_rel_path);


    TestResources::extract_in("raw/", &initial_files_dir).unwrap();
    fs::create_dir_all(file_to_append_abs_path.parent().unwrap()).unwrap();
    fs::create_dir_all(old_file_to_append_abs_path.parent().unwrap()).unwrap();

    // Prepare the 'new' file (its actual ctime will be 'now')
    fs::write(&file_to_append_abs_path, "This is a new file for append clamp_ctime test.").unwrap();
    let old_mtime = SystemTime::now().checked_sub(DURATION_72_HOURS).unwrap_or(UNIX_EPOCH);
    filetime::set_file_mtime(&file_to_append_abs_path, filetime::FileTime::from_system_time(old_mtime)).unwrap();

    // Prepare the 'old' file (set its ctime and mtime to be older than clamp_date)
    // NOTE: Reliably setting ctime is OS-dependent. We set mtime, and ctime might follow.
    // The test relies on this file *actually* having an older ctime on disk *before* append.
    fs::write(&old_file_to_append_abs_path, "This is an old file for append clamp_ctime test.").unwrap();
    let very_old_time = UNIX_EPOCH + Duration::from_secs(1000); // Approx 1970-01-01
     filetime::set_file_times(
        &old_file_to_append_abs_path,
        filetime::FileTime::from_system_time(very_old_time), // atime
        filetime::FileTime::from_system_time(very_old_time), // mtime
    ).unwrap();
    // We hope its ctime is also very_old_time or close. We'll read its actual ctime from disk before append.
    let source_old_file_meta = fs::metadata(&old_file_to_append_abs_path).unwrap();
    let source_old_file_ctime = source_old_file_meta.created().unwrap_or(UNIX_EPOCH);


    // Create an initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &initial_archive_path,
        "--overwrite",
        &initial_files_dir,
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let original_ctimes = get_original_ctimes_from_archive(&initial_archive_path);
    assert!(!original_ctimes.is_empty(), "Original archive for clamp test should have entries");


    let clamp_ctime_str = "2023-03-01T00:00:00Z"; // Date to clamp to
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        &initial_archive_path,
        file_to_append_rel_path,       // This file's ctime is 'now'
        old_file_to_append_rel_path, // This file's ctime is 'very_old_time'
        "--keep-timestamp",
        "--ctime",
        clamp_ctime_str,
        "--clamp-ctime",
        "-C",
        &files_to_append_root_dir,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let expected_clamp_ctime_duration = Duration::from_secs(1677628800); // 2023-03-01T00:00:00Z
    let mut new_appended_entry_checked = false;
    let mut old_appended_entry_checked = false;
    let mut original_entries_checked_count = 0;

    for_each_entry(&initial_archive_path, |entry| {
        let entry_path_str = entry.header().path().as_str();
        let entry_created_duration = entry.metadata().created().expect("Entry should have ctime");

        if entry_path_str == file_to_append_rel_path {
            // This file's natural ctime is 'now' (> clamp_ctime_str).
            // So, it should be clamped to expected_clamp_ctime_duration.
            assert_eq!(
                entry_created_duration, expected_clamp_ctime_duration,
                "Newly appended file '{}' ctime should be clamped to {}, got {:?}",
                entry_path_str, expected_clamp_ctime_duration.as_secs(), entry_created_duration
            );
            new_appended_entry_checked = true;
        } else if entry_path_str == old_file_to_append_rel_path {
            // This file's natural ctime on disk was 'very_old_time' (< clamp_ctime_str).
            // It should retain its older ctime.
            let expected_original_disk_ctime = source_old_file_ctime.duration_since(UNIX_EPOCH).unwrap_or_default();
            assert_eq!(
                entry_created_duration, expected_original_disk_ctime,
                "Old appended file '{}' ctime should be its original disk ctime {:?}, got {:?}",
                entry_path_str, expected_original_disk_ctime, entry_created_duration
            );
            assert!(entry_created_duration < expected_clamp_ctime_duration,
                    "Old appended file '{}' ctime {:?} should be older than clamp ctime {:?}",
                    entry_path_str, entry_created_duration, expected_clamp_ctime_duration);
            old_appended_entry_checked = true;
        } else {
            // Original entries from the initial archive
            if let Some(original_ctime) = original_ctimes.get(entry.header().path()) {
                assert_eq!(
                    &entry_created_duration, original_ctime,
                    "Original entry '{}' ctime changed from {:?} to {:?}",
                    entry_path_str, original_ctime, entry_created_duration
                );
                original_entries_checked_count += 1;
            } else {
                panic!("Original ctime not found for presumably original entry: {}", entry_path_str);
            }
        }
    })
    .unwrap();

    assert!(new_appended_entry_checked, "The new clamped appended entry '{}' was not checked.", file_to_append_rel_path);
    assert!(old_appended_entry_checked, "The old clamped appended entry '{}' was not checked.", old_file_to_append_rel_path);
    assert_eq!(
        original_entries_checked_count,
        original_ctimes.len(),
        "Number of checked original entries in clamp test does not match the initial count."
    );
}
