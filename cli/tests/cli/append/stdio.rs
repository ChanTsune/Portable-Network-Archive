use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, FileEntryBuilder, ReadEntry};
use std::{fs, io::Cursor};

fn assert_appended_archive(bytes: Vec<u8>) {
    let mut archive = Archive::read_header(Cursor::new(bytes)).unwrap();
    let names = archive
        .entries()
        .map(|entry| match entry.unwrap() {
            ReadEntry::Normal(entry) => entry.header().path().as_str().to_owned(),
            ReadEntry::Solid(_) => panic!("append output unexpectedly contained a solid entry"),
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["base.txt", "append_stdio/new.txt"]);
}

#[test]
fn append_rewrite_modes_copy_the_base_before_the_new_entry() {
    setup();
    let _ = fs::remove_dir_all("append_stdio");
    fs::create_dir_all("append_stdio").unwrap();
    fs::write("append_stdio/new.txt", b"new entry").unwrap();

    let mut base = Vec::new();
    let mut archive = Archive::write_header(&mut base).unwrap();
    let entry = FileEntryBuilder::new("base.txt".into())
        .unwrap()
        .build()
        .unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
    fs::write("append_stdio/base.pna", &base).unwrap();

    let file_stdout = cargo_bin_cmd!("pna")
        .args([
            "append",
            "--file",
            "append_stdio/base.pna",
            "append_stdio/new.txt",
        ])
        .output()
        .unwrap();
    assert!(file_stdout.status.success());
    assert_appended_archive(file_stdout.stdout);
    assert_eq!(fs::read("append_stdio/base.pna").unwrap(), base);

    let stdin_stdout = cargo_bin_cmd!("pna")
        .args(["append", "append_stdio/new.txt"])
        .write_stdin(base)
        .output()
        .unwrap();
    assert!(stdin_stdout.status.success());
    assert_appended_archive(stdin_stdout.stdout);

    for (output, overwrite) in [
        ("append_stdio/new.pna", false),
        ("append_stdio/replace.pna", true),
    ] {
        if overwrite {
            fs::write(output, b"old destination").unwrap();
        }
        let mut command = cargo_bin_cmd!("pna");
        command.args([
            "append",
            "--file",
            "append_stdio/base.pna",
            "--output",
            output,
            "append_stdio/new.txt",
        ]);
        if overwrite {
            command.arg("--overwrite");
        }
        command.assert().success();
        assert_appended_archive(fs::read(output).unwrap());
    }
}
