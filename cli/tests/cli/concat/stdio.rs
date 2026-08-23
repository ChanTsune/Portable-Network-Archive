use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use pna::{Archive, ReadEntry};
use portable_network_archive::cli;
use predicates::prelude::*;
use std::{fs, io::Cursor};

fn create_archive(path: &str, input: &str) {
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "--file",
        path,
        "--overwrite",
        input,
    ])
    .unwrap()
    .execute()
    .unwrap();
}

#[test]
fn concat_repeated_file_inputs_to_stdout_in_order() {
    setup();
    fs::create_dir_all("concat_stdio").unwrap();
    fs::write("concat_stdio/left.txt", b"left").unwrap();
    fs::write("concat_stdio/right.txt", b"right").unwrap();
    create_archive("concat_stdio/left.pna", "concat_stdio/left.txt");
    create_archive("concat_stdio/right.pna", "concat_stdio/right.txt");

    let output = cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "concat",
            "--file",
            "concat_stdio/left.pna",
            "--file",
            "concat_stdio/right.pna",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "concat failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut archive = Archive::read_header(Cursor::new(output.stdout)).unwrap();
    let names = archive
        .entries()
        .map(|entry| match entry.unwrap() {
            ReadEntry::Normal(entry) => entry.header().path().to_string(),
            ReadEntry::Solid(_) => panic!("test input unexpectedly produced a solid entry"),
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["concat_stdio/left.txt", "concat_stdio/right.txt"]);
}

#[test]
fn concat_stdin_to_explicit_output() {
    setup();
    fs::create_dir_all("concat_stdin").unwrap();
    fs::write("concat_stdin/input.txt", b"input").unwrap();
    create_archive("concat_stdin/input.pna", "concat_stdin/input.txt");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "concat",
            "--output",
            "concat_stdin/output.pna",
            "--overwrite",
        ])
        .write_stdin(fs::read("concat_stdin/input.pna").unwrap())
        .assert()
        .success();

    let mut archive =
        Archive::read_header(fs::File::open("concat_stdin/output.pna").unwrap()).unwrap();
    let names = archive
        .entries()
        .map(|entry| match entry.unwrap() {
            ReadEntry::Normal(entry) => entry.header().path().to_string(),
            ReadEntry::Solid(_) => panic!("test input unexpectedly produced a solid entry"),
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["concat_stdin/input.txt"]);
}

#[test]
fn concat_rejects_overwrite_without_output() {
    setup();

    cargo_bin_cmd!("pna")
        .args(["concat", "--overwrite"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--overwrite requires --output PATH for concat",
        ));
}
