use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use pna::Archive;
use std::{fs, io::Cursor};

fn assert_readable_archive(bytes: Vec<u8>) {
    let mut archive = Archive::read_header(Cursor::new(bytes)).unwrap();
    let entries = archive
        .entries()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();
    assert!(!entries.is_empty());
}

#[test]
fn strip_streams_file_or_stdin_sources_to_stdout_without_mutating_the_file() {
    setup();
    let _ = fs::remove_dir_all("strip_stdout");
    TestResources::extract_in("zstd_keep_all.pna", "strip_stdout/").unwrap();
    let source = "strip_stdout/zstd_keep_all.pna";
    let original = fs::read(source).unwrap();

    let file_output = cargo_bin_cmd!("pna")
        .args(["strip", "--file", source])
        .output()
        .unwrap();
    assert!(file_output.status.success());
    assert_readable_archive(file_output.stdout);
    assert_eq!(fs::read(source).unwrap(), original);

    let stdin_output = cargo_bin_cmd!("pna")
        .arg("strip")
        .write_stdin(original)
        .output()
        .unwrap();
    assert!(stdin_output.status.success());
    assert_readable_archive(stdin_output.stdout);
}

#[test]
fn strip_create_new_and_replace_destinations_follow_overwrite_intent() {
    setup();
    let _ = fs::remove_dir_all("strip_destinations");
    TestResources::extract_in("zstd_keep_all.pna", "strip_destinations/").unwrap();
    let source = "strip_destinations/zstd_keep_all.pna";
    let create_new = "strip_destinations/new.pna";
    let replace = "strip_destinations/replace.pna";
    fs::write(replace, b"old destination").unwrap();

    cargo_bin_cmd!("pna")
        .args(["strip", "--file", source, "--output", create_new])
        .assert()
        .success();
    cargo_bin_cmd!("pna")
        .args([
            "strip",
            "--file",
            source,
            "--output",
            replace,
            "--overwrite",
        ])
        .assert()
        .success();

    assert_readable_archive(fs::read(create_new).unwrap());
    assert_readable_archive(fs::read(replace).unwrap());
}

#[test]
fn strip_overwrite_commits_in_place() {
    setup();
    let _ = fs::remove_dir_all("strip_in_place");
    TestResources::extract_in("zstd_keep_all.pna", "strip_in_place/").unwrap();
    let source = "strip_in_place/zstd_keep_all.pna";
    let original = fs::read(source).unwrap();

    cargo_bin_cmd!("pna")
        .args(["strip", "--file", source, "--overwrite"])
        .assert()
        .success();

    let rewritten = fs::read(source).unwrap();
    assert_ne!(rewritten, original);
    assert_readable_archive(rewritten);
}
