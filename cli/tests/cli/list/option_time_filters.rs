use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, Metadata, WriteOptions};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, SystemTime},
};

/// Test that --newer-mtime filters list output
#[test]
fn list_with_newer_mtime_filter() {
    setup();
    let test_dir = PathBuf::from("list_newer_mtime_filter");
    let archive_path = test_dir.join("archive.pna");
    fs::create_dir_all(&test_dir).unwrap();

    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let now = SystemTime::now();
    let old_time = now - Duration::from_secs(60 * 60 * 24 * 365);
    let recent_time = now - Duration::from_secs(60 * 60);

    // Old file
    let mut old_metadata = Metadata::default();
    old_metadata.set_modified_time(Some(old_time));
    let mut builder =
        EntryBuilder::new_file_with_metadata("old.txt".into(), old_metadata, WriteOptions::store())
            .unwrap();
    builder.write_all(b"old content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    // Recent file
    let mut recent_metadata = Metadata::default();
    recent_metadata.set_modified_time(Some(recent_time));
    let mut builder = EntryBuilder::new_file_with_metadata(
        "recent.txt".into(),
        recent_metadata,
        WriteOptions::store(),
    )
    .unwrap();
    builder.write_all(b"recent content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    // List with filter: only files newer than 2 hours ago
    let cutoff_time = now - Duration::from_secs(60 * 60 * 2);
    let cutoff_str = format_systemtime_for_cli(cutoff_time);

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "list",
            "--unstable",
            "--newer-mtime",
            &cutoff_str,
            "-f",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert!(!output_str.contains("old.txt"));
    assert!(output_str.contains("recent.txt"));
}

/// Test that --older-mtime filters list output
#[test]
fn list_with_older_mtime_filter() {
    setup();
    let test_dir = PathBuf::from("list_older_mtime_filter");
    let archive_path = test_dir.join("archive.pna");
    fs::create_dir_all(&test_dir).unwrap();

    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let now = SystemTime::now();
    let old_time = now - Duration::from_secs(60 * 60 * 24 * 365);
    let recent_time = now - Duration::from_secs(60 * 60);

    // Old file
    let mut old_metadata = Metadata::default();
    old_metadata.set_modified_time(Some(old_time));
    let mut builder =
        EntryBuilder::new_file_with_metadata("old.txt".into(), old_metadata, WriteOptions::store())
            .unwrap();
    builder.write_all(b"old content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    // Recent file
    let mut recent_metadata = Metadata::default();
    recent_metadata.set_modified_time(Some(recent_time));
    let mut builder = EntryBuilder::new_file_with_metadata(
        "recent.txt".into(),
        recent_metadata,
        WriteOptions::store(),
    )
    .unwrap();
    builder.write_all(b"recent content").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    // List with filter: only files older than 2 hours ago
    let cutoff_time = now - Duration::from_secs(60 * 60 * 2);
    let cutoff_str = format_systemtime_for_cli(cutoff_time);

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "list",
            "--unstable",
            "--older-mtime",
            &cutoff_str,
            "-f",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains("old.txt"));
    assert!(!output_str.contains("recent.txt"));
}

/// Test combining time filters with fast-read for list
#[test]
fn list_with_time_filter_and_fast_read() {
    setup();
    let test_dir = PathBuf::from("list_time_filter_fast_read");
    let archive_path = test_dir.join("archive.pna");
    fs::create_dir_all(&test_dir).unwrap();

    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let now = SystemTime::now();
    let old_time = now - Duration::from_secs(60 * 60 * 24 * 365);
    let recent_time = now - Duration::from_secs(60 * 60);

    // Create multiple entries with different times
    for (name, content, mtime) in [
        ("old.txt", b"old content" as &[u8], old_time),
        ("recent1.txt", b"recent1", recent_time),
        ("recent2.txt", b"recent2", recent_time),
        ("another_old.txt", b"old2", old_time),
    ] {
        let mut metadata = Metadata::default();
        metadata.set_modified_time(Some(mtime));
        let mut builder = EntryBuilder::new_file_with_metadata(
            name.into(),
            metadata,
            WriteOptions::store(),
        )
        .unwrap();
        builder.write_all(content).unwrap();
        archive.add_entry(builder.build().unwrap()).unwrap();
    }

    archive.finalize().unwrap();

    // List with fast-read and newer-mtime filter, specific file operand
    let cutoff_time = now - Duration::from_secs(60 * 60 * 2);
    let cutoff_str = format_systemtime_for_cli(cutoff_time);

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "list",
            "--unstable",
            "--fast-read",
            "--newer-mtime",
            &cutoff_str,
            "-f",
            archive_path.to_str().unwrap(),
            "recent1.txt",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    // Should only show recent1.txt (matches time filter and operand)
    assert!(output_str.contains("recent1.txt"));
    // Should not show others
    assert!(!output_str.contains("old.txt"));
    assert!(!output_str.contains("recent2.txt"));
    assert!(!output_str.contains("another_old.txt"));
}

/// Test that time filter with no_recursive mode works
#[test]
fn list_time_filter_with_no_recursive() {
    setup();
    let test_dir = PathBuf::from("list_time_filter_no_recursive");
    let archive_path = test_dir.join("archive.pna");
    fs::create_dir_all(&test_dir).unwrap();

    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let now = SystemTime::now();
    let old_time = now - Duration::from_secs(60 * 60 * 24 * 365);
    let recent_time = now - Duration::from_secs(60 * 60);

    // Create directory structure
    for (name, mtime) in [
        ("dir/old.txt", old_time),
        ("dir/recent.txt", recent_time),
        ("dir/subdir/file.txt", recent_time),
    ] {
        let mut metadata = Metadata::default();
        metadata.set_modified_time(Some(mtime));
        let mut builder = EntryBuilder::new_file_with_metadata(
            name.into(),
            metadata,
            WriteOptions::store(),
        )
        .unwrap();
        builder.write_all(b"content").unwrap();
        archive.add_entry(builder.build().unwrap()).unwrap();
    }

    archive.finalize().unwrap();

    // List with no-recursive and newer-mtime filter
    let cutoff_time = now - Duration::from_secs(60 * 60 * 2);
    let cutoff_str = format_systemtime_for_cli(cutoff_time);

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "list",
            "--unstable",
            "--no-recursive",
            "--newer-mtime",
            &cutoff_str,
            "-f",
            archive_path.to_str().unwrap(),
            "dir/recent.txt",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains("dir/recent.txt"));
    assert!(!output_str.contains("dir/old.txt"));
    // With --no-recursive, "dir/recent.txt" pattern shouldn't match subdir
    assert!(!output_str.contains("dir/subdir/file.txt"));
}

fn format_systemtime_for_cli(time: SystemTime) -> String {
    use chrono::{DateTime, Local};
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%dT%H:%M:%S").to_string()
}