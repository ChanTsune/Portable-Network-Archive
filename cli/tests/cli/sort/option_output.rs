use crate::utils::{archive::for_each_entry, setup};
use clap::Parser;
use pna::{Archive, FileEntryBuilder};
use portable_network_archive::cli;
use std::fs;

fn write_unsorted(path: &str) {
    let parent = std::path::Path::new(path).parent().unwrap();
    fs::create_dir_all(parent).unwrap();
    let file = fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    let entry_b = FileEntryBuilder::new("b.txt".into()).unwrap();
    archive.add_entry(entry_b.build().unwrap()).unwrap();
    let entry_a = FileEntryBuilder::new("a.txt".into()).unwrap();
    archive.add_entry(entry_a.build().unwrap()).unwrap();
    archive.finalize().unwrap();
}

fn entry_names(path: &str) -> Vec<String> {
    let mut names = Vec::new();
    for_each_entry(path, |e| {
        names.push(e.header().path().as_str().to_string());
    })
    .unwrap();
    names
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run `pna sort` with `--output` pointing at the existing file.
/// Expectation: The command fails without touching either file.
#[test]
fn sort_output_without_overwrite_refuses_to_clobber() {
    setup();
    write_unsorted("sort_overwrite/unsorted.pna");
    fs::write("sort_overwrite/out.pna", b"sentinel").unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "-f",
        "sort_overwrite/unsorted.pna",
        "--output",
        "sort_overwrite/out.pna",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    assert!(format!("{error:?}").contains("already exists"));
    assert_eq!(fs::read("sort_overwrite/out.pna").unwrap(), b"sentinel");
    assert_eq!(
        entry_names("sort_overwrite/unsorted.pna"),
        ["b.txt", "a.txt"]
    );
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run the same sort with `--overwrite`.
/// Expectation: The output holds sorted entries; the original keeps its order.
#[test]
fn sort_output_with_overwrite_replaces() {
    setup();
    write_unsorted("sort_overwrite_ok/unsorted.pna");
    fs::write("sort_overwrite_ok/out.pna", b"sentinel").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "-f",
        "sort_overwrite_ok/unsorted.pna",
        "--output",
        "sort_overwrite_ok/out.pna",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(entry_names("sort_overwrite_ok/out.pna"), ["a.txt", "b.txt"]);
    assert_eq!(
        entry_names("sort_overwrite_ok/unsorted.pna"),
        ["b.txt", "a.txt"]
    );
}
