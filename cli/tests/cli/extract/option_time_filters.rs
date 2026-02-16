use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, Metadata, WriteOptions};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, SystemTime},
};

/// Test that --newer-mtime filters out old entries during extraction
#[test]
fn extract_with_newer_mtime_filter() {
    setup();
    let test_dir = PathBuf::from("extract_newer_mtime_filter");
    let archive_path = test_dir.join("archive.pna");
    fs::create_dir_all(&test_dir).unwrap();

    // Create archive with entries of different mtimes
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let now = SystemTime::now();
    let old_time = now - Duration::from_secs(60 * 60 * 24 * 365); // 1 year ago
    let recent_time = now - Duration::from_secs(60 * 60); // 1 hour ago

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

    // Extract with filter: only files newer than 2 hours ago
    let cutoff_time = now - Duration::from_secs(60 * 60 * 2);
    let cutoff_str = format_systemtime_for_cli(cutoff_time);
    let out_dir = test_dir.join("out");

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "extract",
            "--unstable",
            "--newer-mtime",
            &cutoff_str,
            "--overwrite",
            "--out-dir",
            out_dir.to_str().unwrap(),
            "-f",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Only recent.txt should be extracted
    assert!(!out_dir.join("old.txt").exists());
    assert!(out_dir.join("recent.txt").exists());
    assert_eq!(
        "recent content",
        fs::read_to_string(out_dir.join("recent.txt")).unwrap()
    );
}

/// Test that --older-mtime filters out new entries during extraction
#[test]
fn extract_with_older_mtime_filter() {
    setup();
    let test_dir = PathBuf::from("extract_older_mtime_filter");
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

    // Extract with filter: only files older than 2 hours ago
    let cutoff_time = now - Duration::from_secs(60 * 60 * 2);
    let cutoff_str = format_systemtime_for_cli(cutoff_time);
    let out_dir = test_dir.join("out");

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "extract",
            "--unstable",
            "--older-mtime",
            &cutoff_str,
            "--overwrite",
            "--out-dir",
            out_dir.to_str().unwrap(),
            "-f",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Only old.txt should be extracted
    assert!(out_dir.join("old.txt").exists());
    assert!(!out_dir.join("recent.txt").exists());
    assert_eq!(
        "old content",
        fs::read_to_string(out_dir.join("old.txt")).unwrap()
    );
}

/// Test combining time filters with fast-read
#[test]
fn extract_with_time_filter_and_fast_read() {
    setup();
    let test_dir = PathBuf::from("extract_time_filter_fast_read");
    let archive_path = test_dir.join("archive.pna");
    fs::create_dir_all(&test_dir).unwrap();

    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let now = SystemTime::now();
    let old_time = now - Duration::from_secs(60 * 60 * 24 * 365);
    let recent_time = now - Duration::from_secs(60 * 60);

    // Create duplicates with different times
    for (name, content, mtime) in [
        ("file.txt", b"first-old" as &[u8], old_time),
        ("file.txt", b"second-recent", recent_time),
        ("other.txt", b"other-content", recent_time),
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

    // Extract with fast-read and newer-mtime filter
    let cutoff_time = now - Duration::from_secs(60 * 60 * 2);
    let cutoff_str = format_systemtime_for_cli(cutoff_time);
    let out_dir = test_dir.join("out");

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "extract",
            "--unstable",
            "--fast-read",
            "--newer-mtime",
            &cutoff_str,
            "--overwrite",
            "--out-dir",
            out_dir.to_str().unwrap(),
            "-f",
            archive_path.to_str().unwrap(),
            "file.txt",
        ])
        .assert()
        .success();

    // Should stop after finding first matching file.txt (second-recent)
    assert!(out_dir.join("file.txt").exists());
    assert_eq!(
        "second-recent",
        fs::read_to_string(out_dir.join("file.txt")).unwrap()
    );
    // other.txt should not be extracted (not in operands)
    assert!(!out_dir.join("other.txt").exists());
}

fn format_systemtime_for_cli(time: SystemTime) -> String {
    use chrono::{DateTime, Local};
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%dT%H:%M:%S").to_string()
}