use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

/// Precondition: Archive contains a file whose content changes but keeps the same size.
/// Action: Run diff with `--format jsonl`.
/// Expectation: Every stdout line parses as JSON; exactly one record reports `kind=="content"`
/// for the changed path, without a `target` field.
#[test]
fn diff_with_format_jsonl_and_content_difference() {
    setup();
    let dir = "diff_format_jsonl_content_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::write(&file_path, "new-a").unwrap();

    let assert = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
        ])
        .assert()
        .code(1)
        .stderr("");

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let records: Vec<serde_json::Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    let content_records: Vec<_> = records
        .iter()
        .filter(|record| record["kind"] == "content")
        .collect();
    assert_eq!(content_records.len(), 1);
    assert_eq!(content_records[0]["path"], file_path);
    assert!(content_records[0].get("target").is_none());
}

/// Precondition: Archive contains a file removed from the filesystem afterwards.
/// Action: Run diff with `--format jsonl`.
/// Expectation: A record reports `kind=="missing"` for the removed path.
#[test]
fn diff_with_format_jsonl_and_missing_file() {
    setup();
    let dir = "diff_format_jsonl_missing_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::remove_file(&file_path).unwrap();

    let assert = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
        ])
        .assert()
        .code(1);

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let has_missing = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .any(|record| record["kind"] == "missing");
    assert!(has_missing);
}

/// Precondition: Archive contains a hardlink whose filesystem counterpart is later replaced
/// by an independent file with matching content.
/// Action: Run diff with `--format jsonl`.
/// Expectation: A record reports `kind=="hardlink"` with a non-empty `target` field.
#[cfg(unix)]
#[test]
fn diff_with_format_jsonl_and_broken_hardlink() {
    setup();
    let dir = "diff_format_jsonl_hardlink_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let orig = format!("{dir}/orig.txt");
    let link = format!("{dir}/link.txt");
    fs::write(&orig, "content").unwrap();
    fs::hard_link(&orig, &link).unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &orig, &link])
        .assert()
        .success();

    fs::remove_file(&link).unwrap();
    fs::write(&link, "content").unwrap();

    let assert = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
        ])
        .assert()
        .code(1);

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let hardlink_record = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|record| record["kind"] == "hardlink")
        .expect("a hardlink record");
    assert!(
        hardlink_record["target"]
            .as_str()
            .is_some_and(|target| !target.is_empty())
    );
}

/// Precondition: The filesystem tree matches the archive exactly.
/// Action: Run diff with `--format jsonl`.
/// Expectation: No differences are reported.
#[test]
fn diff_with_format_jsonl_without_differences() {
    setup();
    let dir = "diff_format_jsonl_no_diff_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
        ])
        .assert()
        .success()
        .stdout("");
}

/// Precondition: Archive contains a file whose content changes but keeps the same size.
/// Action: Run diff with `--format plain`.
/// Expectation: Output matches the default tar-style text.
#[test]
fn diff_with_format_plain() {
    setup();
    let dir = "diff_format_plain_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::write(&file_path, "new-a").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "plain",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Contents differ"));
}
