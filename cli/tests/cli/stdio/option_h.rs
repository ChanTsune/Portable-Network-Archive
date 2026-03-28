use crate::utils::{
    archive::{for_each_entry, read_symlink_target},
    setup,
};
use assert_cmd::cargo::cargo_bin_cmd;
use pna::DataKind;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn normalize_link_target(target: &str) -> String {
    #[cfg(windows)]
    {
        target.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        target.to_string()
    }
}

fn normalize_link_path(path: &Path) -> String {
    normalize_link_target(&path.to_string_lossy())
}

fn init_h_upper_resource(dir: &Path) {
    if dir.exists() {
        fs::remove_dir_all(dir).unwrap();
    }
    fs::create_dir_all(dir.join("d1")).unwrap();
    fs::write(dir.join("d1/file1"), b"d1/file1").unwrap();
    fs::write(dir.join("d1/file2"), b"d1/file2").unwrap();
    pna::fs::symlink(Path::new("d1"), dir.join("ld1")).unwrap();
    pna::fs::symlink(Path::new("file1"), dir.join("d1/link1")).unwrap();
    pna::fs::symlink(Path::new("fileX"), dir.join("d1/linkX")).unwrap();
    pna::fs::symlink(Path::new("d1/file2"), dir.join("link2")).unwrap();
    pna::fs::symlink(Path::new("d1/fileY"), dir.join("linkY")).unwrap();
}

fn archive_entries(path: &Path) -> HashMap<String, (DataKind, Option<String>)> {
    let mut entries = HashMap::new();
    for_each_entry(path, |entry| {
        let kind = entry.header().data_kind();
        let link_target = match kind {
            DataKind::SymbolicLink => Some(normalize_link_target(&read_symlink_target(&entry))),
            _ => None,
        };
        let prev = entries.insert(entry.header().path().to_string(), (kind, link_target));
        assert!(
            prev.is_none(),
            "duplicate archive entry: {}",
            entry.header().path()
        );
    })
    .unwrap();
    entries
}

fn assert_symlink(path: impl AsRef<Path>, target: &str) {
    let path = path.as_ref();
    let meta = fs::symlink_metadata(path).unwrap();
    assert!(
        meta.file_type().is_symlink(),
        "{} should be a symlink",
        path.display()
    );
    assert_eq!(
        normalize_link_path(&fs::read_link(path).unwrap()),
        normalize_link_target(target)
    );
}

/// Precondition: Directory contains files, directories, and symlinks (including broken ones)
/// Action: Create archives with and without -H flag using different source arguments
/// Expectation: Without -H or with -H on non-symlink source, all symlinks remain as symlinks;
///              with -H on symlink arguments, only those specific symlinks are followed
#[test]
fn stdio_option_h_follows_only_command_line_symlinks() {
    setup();

    let base = PathBuf::from("stdio_option_h_upper");
    let input = base.join("in");
    init_h_upper_resource(&input);

    // Sub-test 1: Without -H, all symlinks remain as symlinks
    let test1_archive = base.join("test1.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--overwrite",
            "--unstable",
            "-f",
            test1_archive.to_str().unwrap(),
            "-C",
            input.to_str().unwrap(),
            ".",
        ])
        .assert()
        .success();

    let test1_entries = archive_entries(&test1_archive);
    assert_eq!(
        test1_entries.get("ld1"),
        Some(&(DataKind::SymbolicLink, Some("d1".to_string())))
    );
    assert_eq!(
        test1_entries.get("d1/link1"),
        Some(&(DataKind::SymbolicLink, Some("file1".to_string())))
    );
    assert_eq!(
        test1_entries.get("d1/linkX"),
        Some(&(DataKind::SymbolicLink, Some("fileX".to_string())))
    );
    assert_eq!(
        test1_entries.get("link2"),
        Some(&(DataKind::SymbolicLink, Some("d1/file2".to_string())))
    );
    assert_eq!(
        test1_entries.get("linkY"),
        Some(&(DataKind::SymbolicLink, Some("d1/fileY".to_string())))
    );

    let test1_out = base.join("test1_out");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--overwrite",
            "--unstable",
            "-f",
            test1_archive.to_str().unwrap(),
            "--out-dir",
            test1_out.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_symlink(test1_out.join("ld1"), "d1");
    assert_symlink(test1_out.join("d1/link1"), "file1");
    assert_symlink(test1_out.join("d1/linkX"), "fileX");
    assert_symlink(test1_out.join("link2"), "d1/file2");
    assert_symlink(test1_out.join("linkY"), "d1/fileY");

    // Sub-test 2: -H with "." (not a symlink), all internal symlinks remain
    let test2_archive = base.join("test2.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--overwrite",
            "--unstable",
            "-f",
            test2_archive.to_str().unwrap(),
            "-H",
            "-C",
            input.to_str().unwrap(),
            ".",
        ])
        .assert()
        .success();

    let test2_entries = archive_entries(&test2_archive);
    assert_eq!(
        test2_entries.get("ld1"),
        Some(&(DataKind::SymbolicLink, Some("d1".to_string())))
    );
    assert_eq!(
        test2_entries.get("d1/link1"),
        Some(&(DataKind::SymbolicLink, Some("file1".to_string())))
    );
    assert_eq!(
        test2_entries.get("d1/linkX"),
        Some(&(DataKind::SymbolicLink, Some("fileX".to_string())))
    );
    assert_eq!(
        test2_entries.get("link2"),
        Some(&(DataKind::SymbolicLink, Some("d1/file2".to_string())))
    );
    assert_eq!(
        test2_entries.get("linkY"),
        Some(&(DataKind::SymbolicLink, Some("d1/fileY".to_string())))
    );

    let test2_out = base.join("test2_out");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--overwrite",
            "--unstable",
            "-f",
            test2_archive.to_str().unwrap(),
            "--out-dir",
            test2_out.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_symlink(test2_out.join("ld1"), "d1");
    assert_symlink(test2_out.join("d1/link1"), "file1");
    assert_symlink(test2_out.join("d1/linkX"), "fileX");
    assert_symlink(test2_out.join("link2"), "d1/file2");
    assert_symlink(test2_out.join("linkY"), "d1/fileY");

    // Sub-test 3: -H with named symlink arguments, only those are followed
    let test3_archive = base.join("test3.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--overwrite",
            "--unstable",
            "-f",
            test3_archive.to_str().unwrap(),
            "-H",
            "-C",
            input.to_str().unwrap(),
            "ld1",
            "d1",
            "link2",
            "linkY",
        ])
        .assert()
        .success();

    let test3_entries = archive_entries(&test3_archive);
    assert_eq!(
        test3_entries.get("ld1").map(|it| it.0),
        Some(DataKind::Directory)
    );
    assert_eq!(
        test3_entries.get("d1/link1"),
        Some(&(DataKind::SymbolicLink, Some("file1".to_string())))
    );
    assert_eq!(
        test3_entries.get("d1/linkX"),
        Some(&(DataKind::SymbolicLink, Some("fileX".to_string())))
    );
    assert_eq!(
        test3_entries.get("link2").map(|it| it.0),
        Some(DataKind::File)
    );
    assert_eq!(
        test3_entries.get("linkY"),
        Some(&(DataKind::SymbolicLink, Some("d1/fileY".to_string())))
    );

    let test3_out = base.join("test3_out");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--overwrite",
            "--unstable",
            "-f",
            test3_archive.to_str().unwrap(),
            "--out-dir",
            test3_out.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(
        test3_out.join("ld1").is_dir(),
        "ld1 should be extracted as a directory"
    );
    assert_symlink(test3_out.join("d1/link1"), "file1");
    assert_symlink(test3_out.join("d1/linkX"), "fileX");
    assert_eq!(
        fs::read_to_string(test3_out.join("link2")).unwrap(),
        "d1/file2"
    );
    assert_symlink(test3_out.join("linkY"), "d1/fileY");
}
