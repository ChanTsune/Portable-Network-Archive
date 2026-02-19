use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, Duration, EntryBuilder, WriteOptions, fs as pna_fs};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration as StdDuration, SystemTime};

fn build_archive_with_mtime(path: &PathBuf, name: &str, contents: &str, mtime: Duration) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file(name.into(), WriteOptions::builder().build()).unwrap();
            builder.modified(mtime);
            builder.write_all(contents.as_bytes()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer.finalize().unwrap();
}

fn build_single_file_archive(path: &PathBuf, name: &str, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file(name.into(), WriteOptions::builder().build()).unwrap();
            builder.write_all(contents.as_bytes()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer.finalize().unwrap();
}

#[test]
fn unlink_first_replaces_existing_symlink_file() {
    setup();

    let root = PathBuf::from("stdio_unlink_first_file");
    let archive_path = root.join("archive.pna");
    build_single_file_archive(&archive_path, "file.txt", "updated");

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let outside = root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("original.txt"), "keep me").unwrap();

    let link_path = dist.join("file.txt");
    if link_path.exists() {
        pna_fs::remove_path_all(&link_path).unwrap();
    }
    pna_fs::symlink(outside.join("original.txt"), &link_path).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--unlink-first",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(outside.join("original.txt")).unwrap(),
        "keep me"
    );
    assert_eq!(
        fs::read_to_string(dist.join("file.txt")).unwrap(),
        "updated"
    );
    assert!(
        !fs::symlink_metadata(dist.join("file.txt"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn unlink_first_removes_symlinked_parent_directory() {
    setup();

    let root = PathBuf::from("stdio_unlink_first_parent");
    let archive_path = root.join("archive.pna");
    build_single_file_archive(&archive_path, "dir/file.txt", "payload");

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let outside = root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("marker.txt"), "preserve").unwrap();

    let link_dir = dist.join("dir");
    if link_dir.exists() {
        pna_fs::remove_path_all(&link_dir).unwrap();
    }
    pna_fs::symlink(&outside, &link_dir).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--unlink-first",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    let extracted_dir_meta = fs::symlink_metadata(&link_dir).unwrap();
    assert!(extracted_dir_meta.is_dir());
    assert!(!extracted_dir_meta.file_type().is_symlink());
    assert!(fs::read(dist.join("dir/file.txt")).unwrap() == b"payload");
    assert_eq!(
        fs::read_to_string(outside.join("marker.txt")).unwrap(),
        "preserve"
    );
}

#[test]
fn unlink_first_replaces_existing_regular_file_without_overwrite() {
    setup();

    let root = PathBuf::from("stdio_unlink_first_regular");
    let archive_path = root.join("archive.pna");
    build_single_file_archive(&archive_path, "file.txt", "fresh");

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    fs::write(dist.join("file.txt"), "stale").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--unlink-first",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(dist.join("file.txt")).unwrap(), "fresh");
}

/// Precondition: A regular file already exists at the extraction target.
/// Action: Extract with `-U -k` (unlink-first + keep-old-files).
/// Expectation: The existing file is replaced (bsdtar compat: unlink runs before keep-old check).
#[test]
fn unlink_first_with_keep_old_files_replaces_file() {
    setup();

    let root = PathBuf::from("stdio_unlink_first_keep_old");
    let archive_path = root.join("archive.pna");
    build_single_file_archive(&archive_path, "file.txt", "from_archive");

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    fs::write(dist.join("file.txt"), "existing").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--unlink-first",
            "--keep-old-files",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(dist.join("file.txt")).unwrap(),
        "from_archive",
    );
}

/// Precondition: A regular file on disk has a newer mtime than the archive entry.
/// Action: Extract with `-U --keep-newer-files`.
/// Expectation: The file is replaced regardless of timestamps
///              (bsdtar compat: unlink runs before timestamp comparison).
#[test]
fn unlink_first_with_keep_newer_files_replaces_newer_file() {
    setup();

    let root = PathBuf::from("stdio_unlink_first_keep_newer");
    let archive_path = root.join("archive.pna");
    // Archive entry with very old mtime (1 second after epoch)
    build_archive_with_mtime(
        &archive_path,
        "file.txt",
        "from_archive",
        Duration::seconds(1),
    );

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let target = dist.join("file.txt");
    fs::write(&target, "newer_on_disk").unwrap();

    // Set existing file's mtime far into the future so it's clearly "newer"
    let file = fs::File::options().write(true).open(&target).unwrap();
    file.set_modified(SystemTime::now() + StdDuration::from_secs(24 * 60 * 60))
        .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--unlink-first",
            "--keep-newer-files",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(dist.join("file.txt")).unwrap(),
        "from_archive",
    );
}

/// Precondition: Intermediate directory is a symlink to a real directory.
/// Action: Extract with `-P -U` (follow-symlinks + unlink-first).
/// Expectation: The symlink is preserved and the file is written through it
///              (bsdtar compat: -U skips unlink for directory entries when -P follows symlinks).
#[test]
fn unlink_first_with_follow_symlinks_preserves_symlinked_directory() {
    setup();

    let root = PathBuf::from("stdio_unlink_first_follow_PU");
    let archive_path = root.join("archive.pna");
    // Archive with explicit directory entry + file entry (matches bsdtar archive layout)
    {
        if let Some(parent) = archive_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let file = fs::File::create(&archive_path).unwrap();
        let mut writer = Archive::write_header(file).unwrap();
        writer
            .add_entry(EntryBuilder::new_dir("dir".into()).build().unwrap())
            .unwrap();
        writer
            .add_entry({
                let mut builder =
                    EntryBuilder::new_file("dir/file.txt".into(), WriteOptions::builder().build())
                        .unwrap();
                builder.write_all(b"payload").unwrap();
                builder.build().unwrap()
            })
            .unwrap();
        writer.finalize().unwrap();
    }

    let dist = root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let real_dir = dist.join("real_dir");
    fs::create_dir_all(&real_dir).unwrap();

    let link_dir = dist.join("dir");
    if link_dir.symlink_metadata().is_ok() {
        pna_fs::remove_path_all(&link_dir).unwrap();
    }
    pna_fs::symlink("real_dir", &link_dir).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "--unlink-first",
            "--absolute-paths",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            dist.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Symlink should be preserved (not replaced with a real directory)
    assert!(
        fs::symlink_metadata(&link_dir)
            .unwrap()
            .file_type()
            .is_symlink(),
        "dir should still be a symlink"
    );
    // File should be written through the symlink into real_dir
    assert_eq!(
        fs::read_to_string(real_dir.join("file.txt")).unwrap(),
        "payload"
    );
}
