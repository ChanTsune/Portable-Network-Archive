use crate::utils::{
    archive::{for_each_entry, get_entry_names, get_original_ctimes_from_archive_bytes},
    setup, TestResources,
};
use portable_network_archive::EntryName; // Keep this if get_original_ctimes_from_archive_bytes returns HashMap<EntryName, ...>
use std::{
    collections::HashMap,
    env, fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Command as StdCommand, Stdio as StdStdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DURATION_24_HOURS: Duration = Duration::from_secs(24 * 60 * 60);

#[test]
fn stdio_create_with_ctime() {
    setup();
    let test_dir = "stdio_create_with_ctime";
    let input_dir_path = format!("{}/in/", test_dir);
    let output_archive_file_path = format!("{}/stdio_create_ctime.pna", test_dir); // For verification
    fs::create_dir_all(&input_dir_path).unwrap();

    TestResources::extract_in("raw/images/", &input_dir_path).unwrap();

    let ctime_str = "2023-04-01T00:00:00Z";
    let pna_executable = PathBuf::from(env!("CARGO_BIN_EXE_pna"));

    let mut cmd = StdCommand::new(&pna_executable);
    cmd.arg("stdio")
        .arg("-c")
        .arg("--quiet")
        .arg("--keep-timestamp")
        .arg("--ctime")
        .arg(ctime_str)
        .arg(format!("{}icon.png", input_dir_path))
        .arg(format!("{}icon.svg", input_dir_path))
        .stdin(StdStdio::null())
        .stdout(StdStdio::piped());

    let output = cmd.output().expect("Failed to execute pna stdio create");
    assert!(
        output.status.success(),
        "pna stdio create command failed: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(&output_archive_file_path, &output.stdout)
        .expect("Failed to write stdout to archive file");

    let expected_ctime_duration = Duration::from_secs(1680307200); // 2023-04-01T00:00:00Z
    let mut checked_count = 0;
    for_each_entry(&output_archive_file_path, |entry| {
        assert_eq!(
            entry.metadata().created(),
            Some(expected_ctime_duration),
            "Entry '{}' ctime mismatch",
            entry.header().path().as_str()
        );
        checked_count += 1;
    })
    .unwrap();
    assert_eq!(checked_count, 2, "Expected to check 2 entries.");
}

#[test]
fn stdio_create_with_clamp_ctime() {
    setup();
    let test_dir = "stdio_create_with_clamp_ctime";
    let input_dir_path = format!("{}/in/", test_dir);
    let output_archive_file_path = format!("{}/stdio_create_clamp_ctime.pna", test_dir);
    fs::create_dir_all(&input_dir_path).unwrap();

    TestResources::extract_in("raw/", &input_dir_path).unwrap(); // text.txt, empty.txt etc.

    // Make one file ('old_file.txt') have an old ctime on disk
    let old_file_disk_path = PathBuf::from(&input_dir_path).join("text.txt"); // Use text.txt as old
    fs::write(&old_file_disk_path, "This file is old.").unwrap();
    let ancient_time = UNIX_EPOCH + Duration::from_secs(1000); // Approx 1970
    filetime::set_file_times(
        &old_file_disk_path,
        filetime::FileTime::from_system_time(ancient_time),
        filetime::FileTime::from_system_time(ancient_time),
    )
    .unwrap();
    let source_old_file_meta = fs::metadata(&old_file_disk_path).unwrap();
    let source_old_file_ctime_duration = source_old_file_meta
        .created()
        .unwrap_or(UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap();

    // A 'new' file (empty.txt) will have a recent ctime from extraction/creation
    let new_file_disk_path = PathBuf::from(&input_dir_path).join("empty.txt");

    let ctime_str = "2023-04-01T00:00:00Z"; // Clamp date
    let pna_executable = PathBuf::from(env!("CARGO_BIN_EXE_pna"));

    let mut cmd = StdCommand::new(&pna_executable);
    cmd.arg("stdio")
        .arg("-c")
        .arg("--quiet")
        .arg("--keep-timestamp")
        .arg("--ctime")
        .arg(ctime_str)
        .arg("--clamp-ctime")
        .arg(&old_file_disk_path) // This one is old
        .arg(&new_file_disk_path) // This one is new
        .stdin(StdStdio::null())
        .stdout(StdStdio::piped());

    let output = cmd.output().expect("Failed to execute pna stdio create clamp");
    assert!(
        output.status.success(),
        "pna stdio create clamp command failed: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(&output_archive_file_path, &output.stdout)
        .expect("Failed to write stdout to archive file");

    let clamp_date_duration = Duration::from_secs(1680307200); // 2023-04-01T00:00:00Z
    let mut checked_old = false;
    let mut checked_new = false;
    for_each_entry(&output_archive_file_path, |entry| {
        let entry_created_duration = entry
            .metadata()
            .created()
            .expect("Entry should have ctime");
        let entry_path_str = entry.header().path().as_str();

        if entry_path_str.ends_with("text.txt") {
            // This file's original ctime on disk was ancient_time.
            // So, it should retain its original ctime if older than clamp_date_duration.
            assert_eq!(
                entry_created_duration, source_old_file_ctime_duration,
                "Old file '{}' ctime should be its original disk ctime ({:?}), got {:?}",
                entry_path_str, source_old_file_ctime_duration, entry_created_duration
            );
            assert!(
                entry_created_duration < clamp_date_duration,
                "Old file '{}' ctime {:?} should be older than clamp ctime {:?}",
                entry_path_str, entry_created_duration, clamp_date_duration
            );
            checked_old = true;
        } else if entry_path_str.ends_with("empty.txt") {
            // This file's natural ctime is 'now' (approx > clamp_date_duration).
            // So, it should be clamped to clamp_date_duration.
            assert_eq!(
                entry_created_duration, clamp_date_duration,
                "New file '{}' ctime should be clamped to {:?}, got {:?}",
                entry_path_str, clamp_date_duration, entry_created_duration
            );
            checked_new = true;
        }
    })
    .unwrap();
    assert!(checked_old, "Old file was not checked for clamp ctime.");
    assert!(checked_new, "New file was not checked for clamp ctime.");
}

#[test]
fn stdio_append_with_ctime() {
    setup();
    let test_dir = "stdio_append_with_ctime";
    let initial_archive_files_dir = format!("{}/initial_files/", test_dir);
    let files_to_append_dir = format!("{}/files_to_append/", test_dir);
    let final_output_archive_path = format!("{}/final_stdio_append_ctime.pna", test_dir);

    fs::create_dir_all(&initial_archive_files_dir).unwrap();
    fs::create_dir_all(&files_to_append_dir).unwrap();

    TestResources::extract_in("raw/images/", &initial_archive_files_dir).unwrap();
    let file_to_append_name = "text.txt";
    let file_to_append_path_str = format!("{}{}", files_to_append_dir, file_to_append_name);
    fs::write(&file_to_append_path_str, "Content to append.").unwrap();
    // Set mtime of the file to be appended to something old
    filetime::set_file_mtime(
        &file_to_append_path_str,
        filetime::FileTime::from_system_time(UNIX_EPOCH + DURATION_24_HOURS),
    ).unwrap();


    let pna_exec = PathBuf::from(env!("CARGO_BIN_EXE_pna"));

    // 1. Create initial archive (piped to memory)
    let mut create_cmd = StdCommand::new(&pna_exec);
    create_cmd
        .arg("stdio")
        .arg("-c")
        .arg("--quiet")
        .arg("--keep-timestamp") // Keep original times for initial
        .arg(format!("{}icon.png", initial_archive_files_dir))
        .arg(format!("{}icon.svg", initial_archive_files_dir))
        .stdin(StdStdio::null())
        .stdout(StdStdio::piped());
    let create_output = create_cmd
        .output()
        .expect("Failed to create initial archive for stdio append");
    assert!(
        create_output.status.success(),
        "Initial archive creation failed: {}",
        String::from_utf8_lossy(&create_output.stderr)
    );
    let initial_archive_bytes = create_output.stdout;
    let original_ctimes = get_original_ctimes_from_archive_bytes(&initial_archive_bytes);
    assert_eq!(original_ctimes.len(), 2, "Initial archive should have 2 entries.");


    // 2. Append to the archive using stdio
    let append_ctime_str = "2023-04-02T00:00:00Z";

    let mut append_cmd = StdCommand::new(&pna_exec);
    append_cmd
        .arg("stdio")
        .arg("--append")
        .arg("--quiet")
        .arg("--keep-timestamp")
        .arg("--ctime")
        .arg(append_ctime_str)
        .arg(file_to_append_name) // File to append (relative path)
        .arg("-C")
        .arg(&files_to_append_dir) // Source directory for the file to append
        .stdin(StdStdio::piped())
        .stdout(StdStdio::piped());

    let mut process = append_cmd
        .spawn()
        .expect("Failed to spawn pna stdio append");
    let mut stdin = process.stdin.take().expect("Failed to open stdin for append");
    stdin
        .write_all(&initial_archive_bytes)
        .expect("Failed to write initial archive to append stdin");
    drop(stdin); // Close stdin to signal end of input

    let append_output = process
        .wait_with_output()
        .expect("Failed to wait for pna stdio append");
    assert!(
        append_output.status.success(),
        "pna stdio append command failed: stderr: {}",
        String::from_utf8_lossy(&append_output.stderr)
    );
    fs::write(&final_output_archive_path, &append_output.stdout)
        .expect("Failed to write final archive to file");

    let expected_append_ctime_duration = Duration::from_secs(1680393600); // 2023-04-02T00:00:00Z
    let mut appended_entry_checked = false;
    let mut original_entries_checked_count = 0;

    for_each_entry(&final_output_archive_path, |entry| {
        let entry_path_str = entry.header().path().as_str();
        if entry_path_str == file_to_append_name {
            assert_eq!(
                entry.metadata().created(),
                Some(expected_append_ctime_duration)
            );
            appended_entry_checked = true;
        } else {
            let original_ctime = original_ctimes
                .get(entry.header().path())
                .unwrap_or_else(|| {
                    panic!(
                        "Original ctime not found for entry: {}",
                        entry.header().path().as_str()
                    )
                });
            assert_eq!(entry.metadata().created().as_ref(), Some(original_ctime));
            original_entries_checked_count += 1;
        }
    })
    .unwrap();
    assert!(
        appended_entry_checked,
        "Appended entry '{}' was not found or checked.",
        file_to_append_name
    );
    assert_eq!(original_entries_checked_count, original_ctimes.len());
}

#[test]
fn stdio_append_with_clamp_ctime() {
    setup();
    let test_dir = "stdio_append_with_clamp_ctime";
    let initial_archive_files_dir = format!("{}/initial_files/", test_dir);
    let files_to_append_dir = format!("{}/files_to_append/", test_dir);
    let final_output_archive_path = format!("{}/final_stdio_append_clamp_ctime.pna", test_dir);

    fs::create_dir_all(&initial_archive_files_dir).unwrap();
    fs::create_dir_all(&files_to_append_dir).unwrap();

    TestResources::extract_in("raw/images/", &initial_archive_files_dir).unwrap();

    let new_file_to_append_name = "new_text.txt";
    let new_file_to_append_path_str = format!("{}{}", files_to_append_dir, new_file_to_append_name);
    fs::write(&new_file_to_append_path_str, "New content for clamp append.").unwrap();
    // Its ctime will be 'now'

    let old_file_to_append_name = "old_text.txt";
    let old_file_to_append_path_str = format!("{}{}", files_to_append_dir, old_file_to_append_name);
    fs::write(&old_file_to_append_path_str, "Old content for clamp append.").unwrap();
    let ancient_time = UNIX_EPOCH + Duration::from_secs(2000); // Approx 1970
    filetime::set_file_times(
        &old_file_to_append_path_str,
        filetime::FileTime::from_system_time(ancient_time),
        filetime::FileTime::from_system_time(ancient_time),
    ).unwrap();
    let source_old_appended_file_meta = fs::metadata(&old_file_to_append_path_str).unwrap();
    let source_old_appended_file_ctime_duration = source_old_appended_file_meta
        .created()
        .unwrap_or(UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap();


    let pna_exec = PathBuf::from(env!("CARGO_BIN_EXE_pna"));

    // 1. Create initial archive (piped to memory)
    let mut create_cmd = StdCommand::new(&pna_exec);
    create_cmd.arg("stdio").arg("-c").arg("--quiet").arg("--keep-timestamp")
        .arg(format!("{}icon.png", initial_archive_files_dir))
        .stdin(StdStdio::null()).stdout(StdStdio::piped());
    let create_output = create_cmd.output().expect("Failed to create initial archive for stdio append clamp");
    assert!(create_output.status.success(), "Initial archive creation failed (clamp): {}", String::from_utf8_lossy(&create_output.stderr));
    let initial_archive_bytes = create_output.stdout;
    let original_ctimes = get_original_ctimes_from_archive_bytes(&initial_archive_bytes);
    assert_eq!(original_ctimes.len(), 1, "Initial archive for clamp should have 1 entry.");

    // 2. Append to the archive using stdio
    let append_ctime_str = "2023-04-02T00:00:00Z"; // Clamp date

    let mut append_cmd = StdCommand::new(&pna_exec);
    append_cmd.arg("stdio").arg("--append").arg("--quiet").arg("--keep-timestamp")
        .arg("--ctime").arg(append_ctime_str)
        .arg("--clamp-ctime")
        .arg(new_file_to_append_name) // This one is new
        .arg(old_file_to_append_name) // This one is old
        .arg("-C").arg(&files_to_append_dir)
        .stdin(StdStdio::piped()).stdout(StdStdio::piped());

    let mut process = append_cmd.spawn().expect("Failed to spawn pna stdio append clamp");
    let mut stdin = process.stdin.take().expect("Failed to open stdin for append clamp");
    stdin.write_all(&initial_archive_bytes).expect("Failed to write initial archive to append clamp stdin");
    drop(stdin);

    let append_output = process.wait_with_output().expect("Failed to wait for pna stdio append clamp");
    assert!(append_output.status.success(), "pna stdio append clamp command failed: stderr: {}", String::from_utf8_lossy(&append_output.stderr));
    fs::write(&final_output_archive_path, &append_output.stdout).expect("Failed to write final archive (clamp) to file");

    let expected_clamp_ctime_duration = Duration::from_secs(1680393600); // 2023-04-02T00:00:00Z
    let mut appended_new_entry_checked = false;
    let mut appended_old_entry_checked = false;
    let mut original_entries_checked_count = 0;

    for_each_entry(&final_output_archive_path, |entry| {
        let entry_created_duration = entry.metadata().created().expect("Entry should have ctime");
        let entry_path_str = entry.header().path().as_str();

        if entry_path_str == new_file_to_append_name {
            assert_eq!(entry_created_duration, expected_clamp_ctime_duration,
                       "Appended new file '{}' ctime should be clamped to {:?}, got {:?}",
                       entry_path_str, expected_clamp_ctime_duration, entry_created_duration);
            appended_new_entry_checked = true;
        } else if entry_path_str == old_file_to_append_name {
             assert_eq!(entry_created_duration, source_old_appended_file_ctime_duration,
                       "Appended old file '{}' ctime should be its original disk ctime {:?}, got {:?}",
                       entry_path_str, source_old_appended_file_ctime_duration, entry_created_duration);
            assert!(entry_created_duration < expected_clamp_ctime_duration,
                    "Appended old file '{}' ctime {:?} should be older than clamp ctime {:?}",
                    entry_path_str, entry_created_duration, expected_clamp_ctime_duration);
            appended_old_entry_checked = true;
        } else {
            let original_ctime = original_ctimes.get(entry.header().path()).unwrap();
            assert_eq!(entry.metadata().created().as_ref(), Some(original_ctime));
            original_entries_checked_count += 1;
        }
    }).unwrap();
    assert!(appended_new_entry_checked, "Appended new file for clamp test was not found/checked.");
    assert!(appended_old_entry_checked, "Appended old file for clamp test was not found/checked.");
    assert_eq!(original_entries_checked_count, original_ctimes.len());
}
