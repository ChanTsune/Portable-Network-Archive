use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

#[test]
fn extract_fails_on_huge_symlink_target() {
    setup();

    let base_dir = "extract_fails_on_huge_symlink_target";
    fs::create_dir_all(format!("{base_dir}/out")).unwrap();

    let archive_path = format!("{base_dir}/archive.pna");
    let archive_file = fs::File::create(&archive_path).unwrap();
    let mut archive = pna::Archive::write_header(archive_file).unwrap();

    // MAX_LINK_TARGET_SIZE is 64 KiB (65536 bytes).
    // We create a target that is slightly larger.
    let huge_target = "a".repeat(65536 + 1);
    let entry_name = pna::EntryName::from("huge_link");
    let entry_reference = pna::EntryReference::from_utf8_preserve_root(&huge_target);

    let entry = pna::EntryBuilder::new_symlink(entry_name, entry_reference)
        .unwrap()
        .build()
        .unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("x")
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(format!("{base_dir}/out"))
        .arg("--overwrite");

    let output = cmd.output().unwrap();

    // It should fail because the link target is too long.
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("link target too long"));
}

#[test]
fn extract_fails_on_huge_hardlink_target() {
    setup();

    let base_dir = "extract_fails_on_huge_hardlink_target";
    fs::create_dir_all(format!("{base_dir}/out")).unwrap();

    let archive_path = format!("{base_dir}/archive.pna");
    let archive_file = fs::File::create(&archive_path).unwrap();
    let mut archive = pna::Archive::write_header(archive_file).unwrap();

    // Create a hard link with a huge target name.
    let huge_target = "a".repeat(65536 + 1);
    let entry_name = pna::EntryName::from("huge_hardlink");
    let entry_reference = pna::EntryReference::from_utf8_preserve_root(&huge_target);

    let entry = pna::EntryBuilder::new_hard_link(entry_name, entry_reference)
        .unwrap()
        .build()
        .unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("x")
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(format!("{base_dir}/out"))
        .arg("--overwrite");

    let output = cmd.output().unwrap();

    // It should fail because the link target is too long.
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("link target too long"));
}
