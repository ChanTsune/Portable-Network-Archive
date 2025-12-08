use crate::utils::{archive::for_each_entry, setup};
use clap::Parser;
use pna::{Archive, Duration, EntryBuilder, WriteOptions};
use portable_network_archive::{cli, command::Command};
use std::fs;

/// Precondition: An archive contains entries with distinct atime, mtime, ctime, and name values.
/// Action: Run `pna experimental sort` with all four keys in different orderings.
/// Expectation: Entries are ordered according to the specified key priority.
#[test]
fn sort_by_all_keys() {
    setup();
    fs::create_dir_all("sort_by_all").unwrap();
    let file = fs::File::create("sort_by_all/unsorted.pna").unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    // a.txt: ctime=1000, mtime=3000, atime=2000
    let entry1 = {
        let mut b = EntryBuilder::new_file("a.txt".into(), WriteOptions::store()).unwrap();
        b.created(Duration::seconds(1000));
        b.modified(Duration::seconds(3000));
        b.accessed(Duration::seconds(2000));
        b.build().unwrap()
    };
    // b.txt: ctime=1000, mtime=2000, atime=3000
    let entry2 = {
        let mut b = EntryBuilder::new_file("b.txt".into(), WriteOptions::store()).unwrap();
        b.created(Duration::seconds(1000));
        b.modified(Duration::seconds(2000));
        b.accessed(Duration::seconds(3000));
        b.build().unwrap()
    };
    // c.txt: ctime=2000, mtime=1000, atime=1000
    let entry3 = {
        let mut b = EntryBuilder::new_file("c.txt".into(), WriteOptions::store()).unwrap();
        b.created(Duration::seconds(2000));
        b.modified(Duration::seconds(1000));
        b.accessed(Duration::seconds(1000));
        b.build().unwrap()
    };
    archive.add_entry(entry1).unwrap();
    archive.add_entry(entry2).unwrap();
    archive.add_entry(entry3).unwrap();
    archive.finalize().unwrap();

    // by=atime,mtime,ctime,name
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "sort_by_all/unsorted.pna",
        "--by",
        "atime",
        "--by",
        "mtime",
        "--by",
        "ctime",
        "--by",
        "name",
    ])
    .unwrap()
    .execute()
    .unwrap();
    let mut names = Vec::new();
    for_each_entry("sort_by_all/unsorted.pna", |e| {
        names.push(e.header().path().as_str().to_string());
    })
    .unwrap();
    // atime ascending → mtime ascending → ctime ascending → name ascending
    assert_eq!(names, ["c.txt", "a.txt", "b.txt"]);

    // Reverse order (by=name,ctime,mtime,atime)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "sort_by_all/unsorted.pna",
        "--by",
        "name",
        "--by",
        "ctime",
        "--by",
        "mtime",
        "--by",
        "atime",
    ])
    .unwrap()
    .execute()
    .unwrap();
    let mut names = Vec::new();
    for_each_entry("sort_by_all/unsorted.pna", |e| {
        names.push(e.header().path().as_str().to_string());
    })
    .unwrap();
    // name ascending → ctime ascending → mtime ascending → atime ascending
    assert_eq!(names, ["a.txt", "b.txt", "c.txt"]);
}
