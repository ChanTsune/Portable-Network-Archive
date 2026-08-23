use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, ReadEntry};
use std::{fs, io::Cursor};

fn assert_single_updated_entry(output: std::process::Output) {
    assert!(output.status.success());
    let mut archive = Archive::read_header(Cursor::new(output.stdout)).unwrap();
    let entries = archive
        .entries()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();
    assert!(matches!(
        entries.as_slice(),
        [ReadEntry::Normal(entry)] if entry.header().path().as_str() == "update_stdio/new.txt"
    ));
}

#[test]
fn update_writes_file_or_stdin_base_archives_to_stdout() {
    setup();
    fs::create_dir_all("update_stdio").unwrap();
    fs::write("update_stdio/new.txt", b"new entry").unwrap();

    let mut input = Vec::new();
    Archive::write_header(&mut input)
        .unwrap()
        .finalize()
        .unwrap();
    fs::write("update_stdio/base.pna", &input).unwrap();

    let file_output = cargo_bin_cmd!("pna")
        .args([
            "update",
            "--file",
            "update_stdio/base.pna",
            "update_stdio/new.txt",
        ])
        .output()
        .unwrap();
    assert_single_updated_entry(file_output);
    assert_eq!(fs::read("update_stdio/base.pna").unwrap(), input);

    let stdin_output = cargo_bin_cmd!("pna")
        .args(["update", "update_stdio/new.txt"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert_single_updated_entry(stdin_output);
}
