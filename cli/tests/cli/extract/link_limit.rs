use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

#[test]
fn extract_fails_on_oversized_symlink_target() {
    setup();

    let work_dir = "extract_fails_on_oversized_symlink_target";
    fs::create_dir_all(format!("{}/out", work_dir)).unwrap();

    let archive_path = format!("{}/archive.pna", work_dir);
    let archive_file = fs::File::create(&archive_path).unwrap();
    let mut archive = pna::Archive::write_header(archive_file).unwrap();

    // Create a target larger than 64 KiB
    let oversized_target = "a".repeat(64 * 1024 + 1);
    let entry_name = pna::EntryName::from("large_symlink");
    let entry_reference = pna::EntryReference::from(oversized_target.as_str());

    let entry = pna::EntryBuilder::new_symlink(entry_name, entry_reference)
        .unwrap()
        .build()
        .unwrap();

    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &archive_path,
        "--out-dir",
        &format!("{}/out", work_dir),
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "Expected error for oversized symlink target, but got Ok"
    );
    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("symbolic link target exceeds limit"));
}

#[test]
fn extract_fails_on_oversized_hardlink_target() {
    setup();

    let work_dir = "extract_fails_on_oversized_hardlink_target";
    fs::create_dir_all(format!("{}/out", work_dir)).unwrap();

    let archive_path = format!("{}/archive.pna", work_dir);
    let archive_file = fs::File::create(&archive_path).unwrap();
    let mut archive = pna::Archive::write_header(archive_file).unwrap();

    // Create a target larger than 64 KiB
    let oversized_target = "a".repeat(64 * 1024 + 1);
    let entry_name = pna::EntryName::from("large_hardlink");
    let entry_reference = pna::EntryReference::from(oversized_target.as_str());

    let entry = pna::EntryBuilder::new_hard_link(entry_name, entry_reference)
        .unwrap()
        .build()
        .unwrap();

    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &archive_path,
        "--out-dir",
        &format!("{}/out", work_dir),
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "Expected error for oversized hardlink target, but got Ok"
    );
    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("hard link target exceeds limit"));
}
