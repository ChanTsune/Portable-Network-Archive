use crate::utils::{archive::for_each_entry, setup};
use clap::Parser;
use pna::{Archive, EntryBuilder, WriteOptions};
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn sort_by_name() {
    setup();
    fs::create_dir_all("sort_by_name").unwrap();
    let file = fs::File::create("sort_by_name/unsorted.pna").unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    let entry_b = EntryBuilder::new_file("b.txt".into(), WriteOptions::store()).unwrap();
    archive.add_entry(entry_b.build().unwrap()).unwrap();
    let entry_a = EntryBuilder::new_file("a.txt".into(), WriteOptions::store()).unwrap();
    archive.add_entry(entry_a.build().unwrap()).unwrap();
    archive.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "sort",
        "sort_by_name/unsorted.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut names = Vec::new();
    for_each_entry("sort_by_name/unsorted.pna", |e| {
        names.push(e.header().path().as_str().to_string());
    })
    .unwrap();
    assert_eq!(names, ["a.txt", "b.txt"]);
}
