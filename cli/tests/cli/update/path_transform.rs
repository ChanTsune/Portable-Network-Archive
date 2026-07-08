use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, io::prelude::*, time};

const DURATION_24_HOURS: time::Duration = time::Duration::from_secs(24 * 60 * 60);

fn write_newer(path: &str, content: &[u8]) {
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open(path)
        .unwrap();
    file.write_all(content).unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();
}

fn read_entry_content(archive_path: &str, name: &str) -> Vec<u8> {
    let mut found = None;
    archive::for_each_entry(archive_path, |entry| {
        if *entry.header().path() == name {
            let mut buf = Vec::new();
            entry
                .reader(pna::ReadOptions::with_password::<&[u8]>(None))
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
            found = Some(buf);
        }
    })
    .unwrap();
    found.unwrap_or_else(|| panic!("entry {name} not found in {archive_path}"))
}

/// Precondition: An archive is created without any path transform, so the
/// stored entry name matches the file's original path.
/// Action: Make the source file newer, then run `pna experimental update`
/// with a `-s` substitution that renames the stored path.
/// Expectation: Like bsdtar's `-u`, update matches entries by the original
/// disk path, not by the transformed stored name. The original entry is
/// kept unchanged and a new entry is appended under the transformed name
/// carrying the updated content.
#[test]
fn update_with_substitution_appends_transformed_name() {
    setup();
    let base = "update_with_substitution_appends_transformed_name";
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(format!("{base}/src")).unwrap();
    fs::write(format!("{base}/src/a.txt"), b"old").unwrap();
    let archive_path = format!("{base}/archive.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &archive_path,
        "--overwrite",
        &format!("{base}/src/a.txt"),
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    write_newer(&format!("{base}/src/a.txt"), b"new");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        &archive_path,
        &format!("{base}/src/a.txt"),
        "--keep-timestamp",
        "--unstable",
        "-s",
        "#src/#dst/#",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(&archive_path, |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    let original = format!("{base}/src/a.txt");
    let transformed = format!("{base}/dst/a.txt");
    assert_eq!(
        seen,
        HashSet::from([original.clone(), transformed.clone()]),
        "original entry should be kept and a transformed-name entry appended"
    );
    assert_eq!(read_entry_content(&archive_path, &original), b"old");
    assert_eq!(read_entry_content(&archive_path, &transformed), b"new");
}

/// Precondition: An archive is created with a `-s` substitution already
/// applied, so the stored entry name is the transformed name rather than
/// the file's original disk path.
/// Action: Without modifying the source file, run `pna experimental update`
/// again with the same substitution.
/// Expectation: Update matches entries by the original disk path, so the
/// already-transformed stored name never matches an update target. The file
/// is treated as new and re-appended under the same transformed name,
/// duplicating the entry even though it was not modified.
#[test]
fn update_with_substitution_reappends_entry_that_cannot_match_transformed_name() {
    setup();
    let base = "update_with_substitution_reappends_entry_that_cannot_match_transformed_name";
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(format!("{base}/src")).unwrap();
    fs::write(format!("{base}/src/a.txt"), b"unchanged").unwrap();
    let archive_path = format!("{base}/archive.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &archive_path,
        "--overwrite",
        &format!("{base}/src/a.txt"),
        "--keep-timestamp",
        "--unstable",
        "-s",
        "#src/#dst/#",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        &archive_path,
        &format!("{base}/src/a.txt"),
        "--keep-timestamp",
        "--unstable",
        "-s",
        "#src/#dst/#",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        seen.push(entry.header().path().to_string());
    })
    .unwrap();

    let transformed = format!("{base}/dst/a.txt");
    assert_eq!(
        seen,
        vec![transformed.clone(), transformed],
        "unmatched entry should be duplicated even though the source was not modified"
    );
}

/// Precondition: An archive is created without any path transform.
/// Action: Make the source file newer, then run `pna experimental update`
/// with `--strip-components` removing a leading path element.
/// Expectation: Same matching behavior as with `-s`: matching uses the
/// original disk path, so the original entry is kept and a new entry is
/// appended under the stripped name.
#[test]
fn update_with_strip_components_appends_transformed_name() {
    setup();
    let base = "update_with_strip_components_appends_transformed_name";
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(format!("{base}/src")).unwrap();
    fs::write(format!("{base}/src/a.txt"), b"old").unwrap();
    let archive_path = format!("{base}/archive.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &archive_path,
        "--overwrite",
        &format!("{base}/src/a.txt"),
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    write_newer(&format!("{base}/src/a.txt"), b"new");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        &archive_path,
        &format!("{base}/src/a.txt"),
        "--keep-timestamp",
        "--unstable",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(&archive_path, |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    let original = format!("{base}/src/a.txt");
    let transformed = "src/a.txt".to_string();
    assert_eq!(
        seen,
        HashSet::from([original.clone(), transformed.clone()]),
        "original entry should be kept and a stripped-name entry appended"
    );
    assert_eq!(read_entry_content(&archive_path, &original), b"old");
    assert_eq!(read_entry_content(&archive_path, &transformed), b"new");
}
