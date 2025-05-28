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

const DURATION_24_HOURS: Duration = Duration::from_secs(24 * 60 * 60);
const DURATION_48_HOURS: Duration = Duration::from_secs(48 * 60 * 60);

// Helper function to get original ctimes from an archive
fn get_original_ctimes(archive_path: &str) -> HashMap<EntryName, Duration> {
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
fn archive_update_with_ctime() {
    setup();
    let test_dir = "archive_update_with_ctime";
    let initial_archive_path = format!("{}/initial.pna", test_dir);
    let files_to_update_root_dir = format!("{}/source_files/", test_dir); // Directory where source files for update command are located
    
    // Prepare a directory with a structure similar to what's in the archive
    let file_to_update_rel_path = "raw/text.txt"; // Relative path, same as in archive
    let file_to_update_abs_path = PathBuf::from(&files_to_update_root_dir).join(file_to_update_rel_path);

    // Extract initial set of files for creating the archive and for later update
    TestResources::extract_in("raw/", &files_to_update_root_dir).unwrap();
    fs::create_dir_all(file_to_update_abs_path.parent().unwrap()).unwrap();


    // Create an initial archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &initial_archive_path,
        "--overwrite",
        &files_to_update_root_dir, // Add all files from this dir
        "--keep-timestamp", 
    ])
    .unwrap()
    .execute()
    .unwrap();

    let original_ctimes = get_original_ctimes(&initial_archive_path);

    // Modify the file that will be part of the update operation
    // We set its mtime to something old, so we are sure ctime logic is what's tested
    // and its content to be different to ensure it's a candidate for update.
    fs::write(&file_to_update_abs_path, "updated content for ctime test").unwrap();
    filetime::set_file_mtime(&file_to_update_abs_path, filetime::FileTime::from_system_time(SystemTime::now() - DURATION_48_HOURS)).unwrap();
    
    let update_ctime_str = "2023-02-01T00:00:00Z";
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "update",
        &initial_archive_path,
        file_to_update_rel_path, // Specify only the file to update, relative to -C dir
        "--keep-timestamp",
        "--ctime",
        update_ctime_str,
        "-C", // Change directory to where the source files for update are
        &files_to_update_root_dir,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let expected_update_ctime_duration = Duration::from_secs(1675209600); // 2023-02-01T00:00:00Z
    let mut updated_entry_checked = false;
    let mut other_entries_checked_count = 0;

    for_each_entry(&initial_archive_path, |entry| {
        let entry_path_str = entry.header().path().as_str();
        if entry_path_str == file_to_update_rel_path {
            assert_eq!(entry.metadata().created(), Some(expected_update_ctime_duration),
                       "Updated entry '{}' ctime mismatch", entry_path_str);
            updated_entry_checked = true;
        } else {
            // Check that other entries retained their original ctime
            if let Some(original_ctime) = original_ctimes.get(entry.header().path()) {
                 assert_eq!(entry.metadata().created().as_ref(), Some(original_ctime),
                           "Unchanged entry '{}' ctime should not have changed from {:?}", entry_path_str, original_ctime);
            } else {
                // This case should ideally not happen if all entries had ctime initially
                panic!("Original ctime not found for entry: {}", entry_path_str);
            }
            other_entries_checked_count += 1;
        }
    })
    .unwrap();
    assert!(updated_entry_checked, "The updated entry '{}' was not checked.", file_to_update_rel_path);
    assert!(other_entries_checked_count > 0, "No other entries were checked to ensure their ctimes were preserved.");
}

#[test]
fn archive_update_with_clamp_ctime() {
    setup();
    let test_dir = "archive_update_with_clamp_ctime";
    let initial_archive_path = format!("{}/initial_clamp.pna", test_dir);
    let files_to_update_root_dir = format!("{}/source_files_clamp/", test_dir);
    
    let file_to_update_rel_path = "raw/text.txt"; 
    let file_to_update_abs_path = PathBuf::from(&files_to_update_root_dir).join(file_to_update_rel_path);

    let old_file_rel_path = "raw/parent/child.txt"; // Another file that we'll make "old"
    let old_file_abs_path = PathBuf::from(&files_to_update_root_dir).join(old_file_rel_path);


    TestResources::extract_in("raw/", &files_to_update_root_dir).unwrap();
    fs::create_dir_all(file_to_update_abs_path.parent().unwrap()).unwrap();
    fs::create_dir_all(old_file_abs_path.parent().unwrap()).unwrap();

    // Make 'old_file_abs_path' have an old ctime for the initial archive
    fs::write(&old_file_abs_path, "This is an old file.").unwrap();
    let ancient_time = UNIX_EPOCH + DURATION_24_HOURS; // Approx 1970-01-02
    filetime::set_file_times(
        &old_file_abs_path,
        filetime::FileTime::from_system_time(ancient_time), // atime
        filetime::FileTime::from_system_time(ancient_time), // mtime
    ).unwrap();
    // Note: ctime might be updated by OS upon mtime set. We'll read it after initial archive creation.

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &initial_archive_path,
        "--overwrite",
        &files_to_update_root_dir,
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let original_ctimes = get_original_ctimes(&initial_archive_path);
    let original_old_file_ctime = original_ctimes.get(&EntryName::from(old_file_rel_path)).cloned();


    // Modify the file for update ('file_to_update_abs_path'). Its actual ctime will likely be recent.
    fs::write(&file_to_update_abs_path, "updated for clamp_ctime test").unwrap();
    // Setting mtime to be different from ctime helps isolate which timestamp is affected
    filetime::set_file_mtime(&file_to_update_abs_path, filetime::FileTime::from_system_time(SystemTime::now() - DURATION_48_HOURS)).unwrap();

    let clamp_ctime_str = "2023-02-01T00:00:00Z"; // Clamp to this date
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "update",
        &initial_archive_path,
        file_to_update_rel_path, // Update this file
        old_file_rel_path,       // Also "update" this one, though its content is unchanged, its source ctime might be newer than clamp_ctime_str
        "--keep-timestamp",
        "--ctime",
        clamp_ctime_str,
        "--clamp-ctime",
        "-C",
        &files_to_update_root_dir,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let expected_clamp_ctime_duration = Duration::from_secs(1675209600); // 2023-02-01T00:00:00Z
    let mut updated_entry_checked = false;
    let mut old_entry_checked = false;

    for_each_entry(&initial_archive_path, |entry| {
        let entry_path_str = entry.header().path().as_str();
        let entry_created = entry.metadata().created().expect("Entry must have ctime");

        if entry_path_str == file_to_update_rel_path {
            // This file was modified, its original ctime was likely 'now'.
            // So, it should be clamped to expected_clamp_ctime_duration.
            assert!(entry_created <= expected_clamp_ctime_duration, 
                    "Updated entry '{}' ctime {:?} should be <= {:?}", 
                    entry_path_str, entry_created, expected_clamp_ctime_duration);
            updated_entry_checked = true;
        } else if entry_path_str == old_file_rel_path {
            // This file was *not* modified on disk for the update command, but was listed in update.
            // Its source ctime on disk (after initial 'ancient_time' mtime set) could be ancient or 'now' depending on OS.
            // If its source ctime was > expected_clamp_ctime_duration, it should be clamped.
            // If its source ctime was < expected_clamp_ctime_duration, it should keep that older ctime.
            // The crucial part is that it's <= expected_clamp_ctime_duration if its ctime would have been updated.
            // If the file wasn't actually updated by pna (e.g. because content match), its ctime should be original.
            // This test is tricky because pna might not "update" an entry if its content matches,
            // even if listed in the update command.
            // For simplicity, we'll assume if it was listed, and --ctime is used, it *might* get affected by ctime logic.
            // The most robust check is that its ctime is <= expected_clamp_ctime_duration if it *was* processed for update.
            // Or, it retains its original ctime if it wasn't truly updated.
            if let Some(original_ctime) = original_old_file_ctime {
                 if original_ctime <= expected_clamp_ctime_duration {
                    assert_eq!(entry_created, original_ctime,
                               "Old entry '{}' with original ctime {:?} (<= clamp) should retain it, got {:?}",
                               entry_path_str, original_ctime, entry_created);
                 } else {
                    // Original ctime was newer than clamp, so it should be clamped
                     assert!(entry_created <= expected_clamp_ctime_duration,
                               "Old entry '{}' with original ctime {:?} (> clamp) should be clamped to <= {:?}, got {:?}",
                               entry_path_str, original_ctime, expected_clamp_ctime_duration, entry_created);
                 }
            } else {
                 assert!(entry_created <= expected_clamp_ctime_duration,
                        "Old entry '{}' ctime {:?} should be <= clamp time {:?}",
                        entry_path_str, entry_created, expected_clamp_ctime_duration);
            }
            old_entry_checked = true;
        } else {
            // Other files not part of the update list should retain their original ctime
            if let Some(original_ctime) = original_ctimes.get(entry.header().path()) {
                 assert_eq!(entry.metadata().created().as_ref(), Some(original_ctime),
                           "Uninvolved entry '{}' ctime should not have changed from {:?}", entry_path_str, original_ctime);
            }
        }
    })
    .unwrap();
    assert!(updated_entry_checked, "The primary updated entry '{}' was not checked.", file_to_update_rel_path);
    assert!(old_entry_checked, "The 'old' entry '{}' was not checked for clamp behavior.", old_file_rel_path);
}
