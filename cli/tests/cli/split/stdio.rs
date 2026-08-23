use crate::utils::{diff::assert_dirs_equal, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;
use std::fs;

#[test]
fn split_stdin_requires_output_base_path() {
    setup();

    cargo_bin_cmd!("pna")
        .args(["--quiet", "split", "--max-size", "100kb"])
        .write_stdin([])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "standard input requires --output BASE_PATH",
        ));
}

#[test]
fn split_stdin_to_named_parts_round_trips() {
    setup();
    let input_dir = "split_stdin/in";
    fs::create_dir_all(input_dir).unwrap();
    for i in 0..5 {
        fs::write(format!("{input_dir}/file{i}.txt"), vec![b'A' + i; 20]).unwrap();
    }
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "--file",
        "split_stdin/input.pna",
        "--overwrite",
        "split_stdin/in/file0.txt",
        "split_stdin/in/file1.txt",
        "split_stdin/in/file2.txt",
        "split_stdin/in/file3.txt",
        "split_stdin/in/file4.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "split",
            "--output",
            "named.pna",
            "--out-dir",
            "split_stdin/parts",
            "--overwrite",
            "--max-size",
            "150",
        ])
        .write_stdin(fs::read("split_stdin/input.pna").unwrap())
        .assert()
        .success();

    assert!(fs::exists("split_stdin/parts/named.part1.pna").unwrap());
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "extract",
        "--file",
        "split_stdin/parts/named.part1.pna",
        "--out-dir",
        "split_stdin/out",
        "--overwrite",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert_dirs_equal(input_dir, "split_stdin/out");
}
