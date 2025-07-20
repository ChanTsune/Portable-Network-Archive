use crate::utils::{archive::for_each_entry, setup};
use clap::Parser;
use pna::{Archive, Duration, EntryBuilder, WriteOptions};
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn sort_by_atime() {
    setup();
    fs::create_dir_all("sort_by_atime").unwrap();
    let file = fs::File::create("sort_by_atime/unsorted.pna").unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    let entry1 = {
        let mut b = EntryBuilder::new_file("c.txt".into(), WriteOptions::store()).unwrap();
        b.accessed(Duration::seconds(3000));
        b.build().unwrap()
    };
    let entry2 = {
        let mut b = EntryBuilder::new_file("a.txt".into(), WriteOptions::store()).unwrap();
        b.accessed(Duration::seconds(1000));
        b.build().unwrap()
    };
    let entry3 = {
        let mut b = EntryBuilder::new_file("b.txt".into(), WriteOptions::store()).unwrap();
        b.accessed(Duration::seconds(2000));
        b.build().unwrap()
    };
    archive.add_entry(entry1).unwrap();
    archive.add_entry(entry2).unwrap();
    archive.add_entry(entry3).unwrap();
    archive.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "sort",
        "sort_by_atime/unsorted.pna",
        "--by",
        "atime",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut names = Vec::new();
    for_each_entry("sort_by_atime/unsorted.pna", |e| {
        names.push(e.header().path().as_str().to_string());
    })
    .unwrap();
    assert_eq!(names, ["a.txt", "b.txt", "c.txt"]);
}
