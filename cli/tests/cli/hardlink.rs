use crate::utils::{
    archive::{for_each_entry, read_hardlink_target},
    setup,
};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashMap, fs, path::Path};

fn init_resource<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    let parent_path = path.parent().unwrap_or_else(|| Path::new("."));
    let base_path = parent_path.join("in");
    let base_path = &base_path;
    if base_path.exists() {
        fs::remove_dir_all(base_path).unwrap();
    }
    fs::create_dir_all(base_path).unwrap();
    fs::write(base_path.join("origin.txt"), b"abc").unwrap();
    fs::hard_link(base_path.join("origin.txt"), base_path.join("link.txt")).unwrap();
    fs::create_dir_all(base_path.join("origin")).unwrap();
    fs::write(base_path.join("origin/origin2.txt"), b"def").unwrap();
    fs::create_dir_all(base_path.join("link")).unwrap();
    fs::hard_link(
        base_path.join("origin/origin2.txt"),
        base_path.join("link/link2.txt"),
    )
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &path.to_string_lossy(),
        "--overwrite",
        &base_path.to_string_lossy(),
    ])
    .unwrap()
    .execute()
    .unwrap();
}

fn assert_hardlink_pair(
    kinds: &HashMap<String, pna::DataKind>,
    targets: &HashMap<String, String>,
    a: &str,
    b: &str,
) {
    match (kinds[a], kinds[b]) {
        (pna::DataKind::FILE, pna::DataKind::HARD_LINK) => {
            assert_eq!(targets[b], a);
        }
        (pna::DataKind::HARD_LINK, pna::DataKind::FILE) => {
            assert_eq!(targets[a], b);
        }
        (kind_a, kind_b) => {
            panic!("expected one file and one hard link entry, got {kind_a:?} and {kind_b:?}");
        }
    }
}

/// Precondition: A directory contains two pairs of hard-linked files.
/// Action: Creates an archive from the directory and extracts it.
/// Expectation: One file of each pair is stored as a hard link entry
/// referencing the other, and extraction restores the shared contents.
#[test]
fn hardlink() {
    setup();
    init_resource("hardlink/hardlink.pna");

    let mut kinds = HashMap::new();
    let mut hardlink_targets = HashMap::new();
    for_each_entry("hardlink/hardlink.pna", |entry| {
        let name = entry.header().path().to_string();
        if entry.header().data_kind() == pna::DataKind::HARD_LINK {
            hardlink_targets.insert(name.clone(), read_hardlink_target(&entry));
        }
        kinds.insert(name, entry.header().data_kind());
    })
    .unwrap();

    assert_eq!(kinds.len(), 7);
    assert_eq!(kinds["hardlink/in"], pna::DataKind::DIRECTORY);
    assert_eq!(kinds["hardlink/in/link"], pna::DataKind::DIRECTORY);
    assert_eq!(kinds["hardlink/in/origin"], pna::DataKind::DIRECTORY);
    assert_hardlink_pair(
        &kinds,
        &hardlink_targets,
        "hardlink/in/origin.txt",
        "hardlink/in/link.txt",
    );
    assert_hardlink_pair(
        &kinds,
        &hardlink_targets,
        "hardlink/in/origin/origin2.txt",
        "hardlink/in/link/link2.txt",
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "hardlink/hardlink.pna",
        "--overwrite",
        "--out-dir",
        "hardlink/dist",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        "abc",
        fs::read_to_string("hardlink/dist/hardlink/in/origin.txt").unwrap()
    );

    assert_eq!(
        "abc",
        fs::read_to_string("hardlink/dist/hardlink/in/link.txt").unwrap()
    );

    assert_eq!(
        "def",
        fs::read_to_string("hardlink/dist/hardlink/in/origin/origin2.txt").unwrap()
    );
    assert_eq!(
        "def",
        fs::read_to_string("hardlink/dist/hardlink/in/link/link2.txt").unwrap()
    );
}

fn init_hardlink_archive(dir: &str) -> (String, String) {
    let dir_path = Path::new(dir);
    let _ = fs::remove_dir_all(dir_path);
    fs::create_dir_all(dir_path).unwrap();

    let orig = format!("{dir}/orig.txt");
    let link = format!("{dir}/link.txt");
    fs::write(&orig, "content").unwrap();
    fs::hard_link(&orig, &link).unwrap();

    let archive_path = format!("{dir}/test.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "-f",
        &archive_path,
        "--overwrite",
        &orig,
        &link,
    ])
    .unwrap()
    .execute()
    .unwrap();
    (archive_path, link)
}

fn run_diff(archive_path: &str) -> bool {
    cli::Cli::try_parse_from(["pna", "--quiet", "experimental", "diff", "-f", archive_path])
        .unwrap()
        .execute()
        .is_ok()
}

/// Precondition: Archive contains a hard link whose files are unchanged.
/// Action: Runs `pna experimental diff` against the archive.
/// Expectation: Finds no differences.
#[test]
fn diff_matches_intact_hardlink() {
    setup();
    let (archive_path, _) = init_hardlink_archive("hardlink_diff_matches");
    assert!(run_diff(&archive_path));
}

/// Precondition: Archive contains a hard link.
/// Action: Replaces the link with a regular file, then runs `pna experimental diff`.
/// Expectation: Exits with an error because the files are not linked.
#[test]
fn diff_detects_broken_hardlink() {
    setup();
    let (archive_path, link) = init_hardlink_archive("hardlink_diff_broken");
    fs::remove_file(&link).unwrap();
    fs::write(&link, "content").unwrap();
    assert!(!run_diff(&archive_path));
}

/// Precondition: Archive contains a hard link.
/// Action: Removes the link from the filesystem, then runs `pna experimental diff`.
/// Expectation: Exits with an error because the entry is missing.
#[test]
fn diff_detects_missing_hardlink() {
    setup();
    let (archive_path, link) = init_hardlink_archive("hardlink_diff_missing");
    fs::remove_file(&link).unwrap();
    assert!(!run_diff(&archive_path));
}
