use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::{
    fs,
    io::Write,
    path::Path,
    time::{Duration, SystemTime},
};

fn extract_time(path: impl AsRef<Path>) -> (SystemTime, SystemTime, Option<SystemTime>) {
    let meta = fs::metadata(path).unwrap();
    let modified = meta.modified().unwrap();
    let accessed = meta.accessed().unwrap();
    let created = meta.created().ok();
    (modified, accessed, created)
}

fn assert_same_second(actual: SystemTime, expected: SystemTime, label: &str) {
    let actual = actual
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expected = expected
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(actual, expected, "{label} mismatch");
}

/// Precondition: Create an archive that stores a file with explicitly specified atime/mtime/ctime.
/// Action: Extract via `pna extract` with `--keep-timestamp` into an empty output directory.
/// Expectation: The extracted file has the same timestamps as recorded in the archive.
#[test]
fn extract_keep_timestamp_restores_file_times() {
    setup();

    fs::create_dir_all("extract_keep_timestamp/in").unwrap();
    fs::write("extract_keep_timestamp/in/file.txt", "content").unwrap();

    let atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1704067200);
    let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1704153600);
    #[cfg(any(windows, target_os = "macos"))]
    let ctime = SystemTime::UNIX_EPOCH + Duration::from_secs(1704240000);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "extract_keep_timestamp/archive.pna",
        "--overwrite",
        "extract_keep_timestamp/in/file.txt",
        "--keep-timestamp",
        "--atime",
        "2024-01-01T00:00:00Z",
        "--mtime",
        "2024-01-02T00:00:00Z",
        "--ctime",
        "2024-01-03T00:00:00Z",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_keep_timestamp/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_keep_timestamp/out",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let (modified, accessed, _created) =
        extract_time("extract_keep_timestamp/out/extract_keep_timestamp/in/file.txt");
    assert_same_second(accessed, atime, "atime");
    assert_same_second(modified, mtime, "mtime");

    #[cfg(any(windows, target_os = "macos"))]
    assert_same_second(
        _created.expect("creation time should be available on this platform"),
        ctime,
        "ctime",
    );
}

/// Precondition: Archive contains a directory entry with known mtime and atime.
/// Action: Extract with `--keep-timestamp`.
/// Expectation: The extracted directory's mtime and atime match the archived values.
#[test]
fn extract_keep_timestamp_restores_directory_times() {
    setup();

    let base = "extract_keep_timestamp_restores_directory_times";
    let archive_path = format!("{base}/archive.pna");
    fs::create_dir_all(base).unwrap();

    let atime_epoch = pna::Duration::seconds(1_704_067_200); // 2024-01-01T00:00:00Z
    let mtime_epoch = pna::Duration::seconds(1_704_153_600); // 2024-01-02T00:00:00Z

    // Build archive programmatically with a directory entry having explicit timestamps
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = pna::Archive::write_header(file).unwrap();
    let mut dir_builder = pna::EntryBuilder::new_dir("mydir".into());
    dir_builder.modified(mtime_epoch);
    dir_builder.accessed(atime_epoch);
    archive.add_entry(dir_builder.build().unwrap()).unwrap();
    // Add a file inside the directory so extraction creates the directory
    let mut file_builder =
        pna::EntryBuilder::new_file("mydir/file.txt".into(), pna::WriteOptions::store()).unwrap();
    file_builder.write_all(b"content").unwrap();
    archive.add_entry(file_builder.build().unwrap()).unwrap();
    archive.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &format!("{base}/out"),
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // WASM does not support path-based timestamp restoration (filetime crate
    // limitation); restore_path_timestamps is gated out for wasm targets.
    #[cfg(not(target_family = "wasm"))]
    {
        let atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_704_067_200);
        let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_704_153_600);
        let (modified, accessed, _) = extract_time(format!("{base}/out/mydir"));
        assert_same_second(modified, mtime, "directory mtime");
        assert_same_second(accessed, atime, "directory atime");
    }
}

/// Precondition: Archive contains nested directory entries where each level has a distinct recorded mtime and atime.
/// Action: Extract with `--keep-timestamp`.
/// Expectation: Every directory level, including intermediates, has its archived timestamps on disk.
#[test]
fn extract_with_keep_timestamp_restores_nested_directory_times() {
    setup();

    let base = "extract_with_keep_timestamp_restores_nested_directory_times";
    let archive_path = format!("{base}/archive.pna");
    fs::create_dir_all(base).unwrap();

    let a_mtime_epoch = pna::Duration::seconds(1_577_836_800); // 2020-01-01T00:00:00Z
    let a_atime_epoch = pna::Duration::seconds(1_577_923_200); // 2020-01-02T00:00:00Z
    let b_mtime_epoch = pna::Duration::seconds(1_609_459_200); // 2021-01-01T00:00:00Z
    let b_atime_epoch = pna::Duration::seconds(1_609_545_600); // 2021-01-02T00:00:00Z
    let c_mtime_epoch = pna::Duration::seconds(1_640_995_200); // 2022-01-01T00:00:00Z
    let c_atime_epoch = pna::Duration::seconds(1_641_081_600); // 2022-01-02T00:00:00Z

    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = pna::Archive::write_header(file).unwrap();

    let mut dir_a = pna::EntryBuilder::new_dir("a".into());
    dir_a.modified(a_mtime_epoch);
    dir_a.accessed(a_atime_epoch);
    archive.add_entry(dir_a.build().unwrap()).unwrap();

    let mut dir_b = pna::EntryBuilder::new_dir("a/b".into());
    dir_b.modified(b_mtime_epoch);
    dir_b.accessed(b_atime_epoch);
    archive.add_entry(dir_b.build().unwrap()).unwrap();

    let mut dir_c = pna::EntryBuilder::new_dir("a/b/c".into());
    dir_c.modified(c_mtime_epoch);
    dir_c.accessed(c_atime_epoch);
    archive.add_entry(dir_c.build().unwrap()).unwrap();

    let mut file_builder =
        pna::EntryBuilder::new_file("a/b/c/file.txt".into(), pna::WriteOptions::store()).unwrap();
    file_builder.write_all(b"content").unwrap();
    archive.add_entry(file_builder.build().unwrap()).unwrap();
    archive.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &format!("{base}/out"),
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    #[cfg(not(target_family = "wasm"))]
    {
        let a_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_577_836_800);
        let a_atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_577_923_200);
        let b_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_609_459_200);
        let b_atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_609_545_600);
        let c_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_640_995_200);
        let c_atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_641_081_600);

        let (a_mod, a_acc, _) = extract_time(format!("{base}/out/a"));
        assert_same_second(a_mod, a_mtime, "directory a mtime");
        assert_same_second(a_acc, a_atime, "directory a atime");

        let (b_mod, b_acc, _) = extract_time(format!("{base}/out/a/b"));
        assert_same_second(b_mod, b_mtime, "directory a/b mtime");
        assert_same_second(b_acc, b_atime, "directory a/b atime");

        let (c_mod, c_acc, _) = extract_time(format!("{base}/out/a/b/c"));
        assert_same_second(c_mod, c_mtime, "directory a/b/c mtime");
        assert_same_second(c_acc, c_atime, "directory a/b/c atime");
    }
}

/// Precondition: Archive contains a file entry and a hardlink entry pointing to it, both with known timestamps.
/// Action: Extract with `--keep-timestamp`.
/// Expectation: The hardlink's mtime matches the archived value and shares the same inode as the target.
#[test]
fn extract_keep_timestamp_restores_hardlink_times() {
    setup();

    let base = "extract_keep_timestamp_restores_hardlink_times";
    let archive_path = format!("{base}/archive.pna");
    fs::create_dir_all(base).unwrap();

    let mtime_epoch = pna::Duration::seconds(1_704_067_200); // 2024-01-01T00:00:00Z
    let atime_epoch = pna::Duration::seconds(1_704_153_600); // 2024-01-02T00:00:00Z

    // Build archive: file + hardlink pointing to it, both with timestamps
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = pna::Archive::write_header(file).unwrap();

    let mut file_builder =
        pna::EntryBuilder::new_file("original.txt".into(), pna::WriteOptions::store()).unwrap();
    file_builder.modified(mtime_epoch);
    file_builder.accessed(atime_epoch);
    file_builder.write_all(b"shared content").unwrap();
    archive.add_entry(file_builder.build().unwrap()).unwrap();

    let mut link_builder =
        pna::EntryBuilder::new_hard_link("link.txt".into(), "original.txt".into()).unwrap();
    link_builder.modified(mtime_epoch);
    link_builder.accessed(atime_epoch);
    archive.add_entry(link_builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &format!("{base}/out"),
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // WASM does not support path-based timestamp restoration (filetime crate
    // limitation); restore_path_timestamps is gated out for wasm targets.
    #[cfg(not(target_family = "wasm"))]
    {
        let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_704_067_200);
        let atime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_704_153_600);

        // Verify the hardlink's timestamps
        let (modified, accessed, _) = extract_time(format!("{base}/out/link.txt"));
        assert_same_second(modified, mtime, "hardlink mtime");
        assert_same_second(accessed, atime, "hardlink atime");
    }

    // same_file is not supported on wasi.
    #[cfg(not(target_os = "wasi"))]
    assert!(
        same_file::is_same_file(
            format!("{base}/out/original.txt"),
            format!("{base}/out/link.txt")
        )
        .unwrap(),
        "hardlink should share the same inode as the original file"
    );
}
