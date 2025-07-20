use crate::utils::{archive::for_each_entry, setup};
use clap::Parser;
use pna::{Archive, Duration, EntryBuilder, WriteOptions};
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn sort_by_multiple_keys() {
    setup();
    fs::create_dir_all("sort_by_multi").unwrap();
    let file = fs::File::create("sort_by_multi/unsorted.pna").unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    // a.txt: ctime=1000, mtime=3000
    let entry1 = {
        let mut b = EntryBuilder::new_file("a.txt".into(), WriteOptions::store()).unwrap();
        b.created(Duration::seconds(1000));
        b.modified(Duration::seconds(3000));
        b.build().unwrap()
    };
    // b.txt: ctime=1000, mtime=2000
    let entry2 = {
        let mut b = EntryBuilder::new_file("b.txt".into(), WriteOptions::store()).unwrap();
        b.created(Duration::seconds(1000));
        b.modified(Duration::seconds(2000));
        b.build().unwrap()
    };
    // c.txt: ctime=2000, mtime=1000
    let entry3 = {
        let mut b = EntryBuilder::new_file("c.txt".into(), WriteOptions::store()).unwrap();
        b.created(Duration::seconds(2000));
        b.modified(Duration::seconds(1000));
        b.build().unwrap()
    };
    archive.add_entry(entry1).unwrap();
    archive.add_entry(entry2).unwrap();
    archive.add_entry(entry3).unwrap();
    archive.finalize().unwrap();

    // by=ctime,mtime
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "sort",
        "sort_by_multi/unsorted.pna",
        "--by",
        "ctime",
        "--by",
        "mtime",
    ])
    .unwrap()
    .execute()
    .unwrap();
    let mut names = Vec::new();
    for_each_entry("sort_by_multi/unsorted.pna", |e| {
        names.push(e.header().path().as_str().to_string());
    })
    .unwrap();
    assert_eq!(names, ["b.txt", "a.txt", "c.txt"]);
}
