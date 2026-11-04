use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::{fs, path::Path};

fn create_multipart_archive(dir: &str) -> String {
    fs::create_dir_all(dir).unwrap();
    for name in ["a", "b", "c"] {
        fs::write(format!("{dir}/{name}.txt"), "x".repeat(300)).unwrap();
    }

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--unstable",
            "--split",
            "200",
            &format!("{dir}/a.txt"),
            &format!("{dir}/b.txt"),
            &format!("{dir}/c.txt"),
        ])
        .assert()
        .success();

    assert!(Path::new(&format!("{dir}/test.part2.pna")).exists());
    format!("{dir}/test.part1.pna")
}

/// Precondition: A multipart archive matches the filesystem exactly.
/// Action: Run `pna experimental diff` against the first part.
/// Expectation: Exits successfully and produces no stdout.
#[test]
fn diff_multipart_without_differences() {
    setup();
    let dir = "diff_multipart_without_differences_test";
    let _ = fs::remove_dir_all(dir);
    let part1 = create_multipart_archive(dir);

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &part1])
        .assert()
        .success()
        .stdout("");
}

/// Precondition: A multipart archive contains a file that is later removed from disk.
/// Action: Run `pna experimental diff` against the first part.
/// Expectation: Exits with status 1 and reports the missing file on stdout.
#[test]
fn diff_multipart_with_missing_file() {
    setup();
    let dir = "diff_multipart_with_missing_file_test";
    let _ = fs::remove_dir_all(dir);
    let part1 = create_multipart_archive(dir);

    fs::remove_file(format!("{dir}/a.txt")).unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &part1])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Cannot stat"));
}
