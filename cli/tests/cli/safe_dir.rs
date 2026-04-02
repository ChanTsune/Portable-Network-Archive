use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryBuilder, EntryName, EntryReference, WriteOptions};
use portable_network_archive::cli;
use std::{fs, io::Write};

/// Precondition: Archive contains a symlink entry pointing outside the output directory
///   followed by a file entry whose name traverses through the symlink.
/// Action: Extract with `--out-dir`.
/// Expectation: No file is written outside the output directory (Zip Slip attack is blocked).
#[test]
fn extract_with_symlink_then_file_attack() {
    setup();

    let base = "safe_dir_symlink_attack";
    let out_dir = format!("{base}/out");
    let target_dir = format!("{base}/target");
    let archive_path = format!("{base}/archive.pna");

    // Clean up from previous runs.
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(&out_dir).unwrap();
    fs::create_dir_all(&target_dir).unwrap();

    // Build malicious archive:
    //  1. symlink "link" -> "../../target/"
    //  2. file "link/payload.txt" with content
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let symlink_entry = EntryBuilder::new_symlink(
        EntryName::from_utf8_preserve_root("link"),
        EntryReference::from_utf8_preserve_root("../../target/"),
    )
    .unwrap()
    .build()
    .unwrap();
    archive.add_entry(symlink_entry).unwrap();

    let file_name = EntryName::from_utf8_preserve_root("link/payload.txt");
    let mut file_builder = EntryBuilder::new_file(file_name, WriteOptions::store()).unwrap();
    file_builder.write_all(b"malicious payload").unwrap();
    let file_entry = file_builder.build().unwrap();
    archive.add_entry(file_entry).unwrap();

    archive.finalize().unwrap();

    // Extract -- the operation may succeed or error, but must not write outside out_dir.
    let _ = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &out_dir,
    ])
    .unwrap()
    .execute();

    assert!(
        !fs::exists(format!("{target_dir}/payload.txt")).unwrap(),
        "payload.txt must not escape to the target directory via symlink traversal"
    );
}

/// Precondition: Archive contains a file entry whose stored name includes parent-directory
///   traversal components (e.g. `../../etc/passwd`-style path).
/// Action: Extract with `--out-dir`.
/// Expectation: No file is written outside the output directory.
#[test]
fn extract_with_path_traversal_in_entry_name() {
    setup();

    let base = "safe_dir_path_traversal";
    let out_dir = format!("{base}/out");
    let archive_path = format!("{base}/archive.pna");

    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(&out_dir).unwrap();

    // Build archive with a deeply traversing entry name.
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    let raw_name = EntryName::from_utf8_preserve_root("../../etc/passwd");
    let mut builder = EntryBuilder::new_file(raw_name, WriteOptions::store()).unwrap();
    builder.write_all(b"fake passwd content").unwrap();
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();

    archive.finalize().unwrap();

    // Extract.
    let _ = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &out_dir,
    ])
    .unwrap()
    .execute();

    // The file must not escape out_dir. After sanitization it should land inside out_dir.
    assert!(
        !fs::exists(format!("{base}/etc/passwd")).unwrap(),
        "file must not escape out-dir via ../ traversal"
    );
    // Verify the sanitized file ends up inside out_dir.
    assert!(
        fs::exists(format!("{out_dir}/etc/passwd")).unwrap(),
        "sanitized entry should be extracted inside out-dir"
    );
}

/// Precondition: Archive contains a hardlink entry whose target traverses outside the
///   output directory.
/// Action: Extract with `--out-dir`.
/// Expectation: The hardlink is not created pointing outside the output directory.
#[cfg(unix)]
#[test]
fn extract_with_hardlink_escape() {
    setup();

    let base = "safe_dir_hardlink_escape";
    let out_dir = format!("{base}/out");
    let archive_path = format!("{base}/archive.pna");

    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(&out_dir).unwrap();

    // Create a sentinel file outside out_dir that the hardlink would try to target.
    let sentinel_path = format!("{base}/sentinel.txt");
    fs::write(&sentinel_path, "original sentinel content").unwrap();
    let original_sentinel = fs::read(&sentinel_path).unwrap();

    // Build archive with a hardlink entry targeting outside the output directory.
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    // First add a regular file so there is something in the archive.
    let mut file_builder =
        EntryBuilder::new_file("normal.txt".into(), WriteOptions::store()).unwrap();
    file_builder.write_all(b"normal content").unwrap();
    let file_entry = file_builder.build().unwrap();
    archive.add_entry(file_entry).unwrap();

    // Add hardlink with a target that escapes via ../
    let hardlink_entry = EntryBuilder::new_hard_link(
        EntryName::from_utf8_preserve_root("escaped_link"),
        EntryReference::from_utf8_preserve_root("../sentinel.txt"),
    )
    .unwrap()
    .build()
    .unwrap();
    archive.add_entry(hardlink_entry).unwrap();

    archive.finalize().unwrap();

    // Extract -- may succeed or error, but must not create an escaping hardlink.
    let _ = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &archive_path,
        "--overwrite",
        "--out-dir",
        &out_dir,
    ])
    .unwrap()
    .execute();

    // The sentinel file must remain unchanged (a successful hardlink would share its inode).
    assert_eq!(
        fs::read(&sentinel_path).unwrap(),
        original_sentinel,
        "sentinel file outside out-dir must not be affected by hardlink escape"
    );

    // If the hardlink was created inside out_dir, it must not be a hardlink to the
    // sentinel outside.
    let link_path = format!("{out_dir}/escaped_link");
    if fs::exists(&link_path).unwrap() {
        use std::os::unix::fs::MetadataExt;
        let sentinel_ino = fs::metadata(&sentinel_path).unwrap().ino();
        let link_ino = fs::metadata(&link_path).unwrap().ino();
        assert_ne!(
            sentinel_ino, link_ino,
            "hardlink inside out-dir must not share inode with file outside out-dir"
        );
    }
}
