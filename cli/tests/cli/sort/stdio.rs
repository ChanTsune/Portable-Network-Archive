use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, FileEntryBuilder, ReadEntry};
use std::io::Cursor;

#[test]
fn sort_reads_an_archive_from_stdin_and_writes_the_sorted_archive_to_stdout() {
    setup();
    let mut input = Vec::new();
    let mut archive = Archive::write_header(&mut input).unwrap();
    for name in ["b.txt", "a.txt"] {
        let entry = FileEntryBuilder::new(name.into()).unwrap().build().unwrap();
        archive.add_entry(entry).unwrap();
    }
    archive.finalize().unwrap();

    let output = cargo_bin_cmd!("pna")
        .arg("sort")
        .write_stdin(input)
        .output()
        .unwrap();

    assert!(output.status.success());
    let mut archive = Archive::read_header(Cursor::new(output.stdout)).unwrap();
    let names = archive
        .entries()
        .map(|entry| match entry.unwrap() {
            ReadEntry::Normal(entry) => entry.header().path().as_str().to_owned(),
            ReadEntry::Solid(_) => panic!("sort output unexpectedly contained a solid entry"),
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["a.txt", "b.txt"]);
}
