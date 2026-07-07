use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs, io::prelude::*, time};

const DURATION_24_HOURS: time::Duration = time::Duration::from_secs(24 * 60 * 60);

fn write_newer(path: &str, content: &[u8]) {
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open(path)
        .unwrap();
    file.write_all(content).unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();
}

/// Precondition: An archive contains multiple files and all of them are
/// modified with newer mtimes.
/// Action: Run `pna experimental update` on the files with `--include`
/// matching one of them. Note that include patterns are matched against
/// each visited path (bsdtar-compatible), so a directory whose name does
/// not match the pattern is not descended into.
/// Expectation: Only the included file is updated; entries outside the
/// include pattern keep their original content.
#[test]
fn update_with_include() {
    setup();
    let _ = fs::remove_dir_all("update_with_include");
    fs::create_dir_all("update_with_include/in").unwrap();
    fs::write("update_with_include/in/a.txt", b"old-a").unwrap();
    fs::write("update_with_include/in/b.txt", b"old-b").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_with_include/archive.pna",
        "--overwrite",
        "update_with_include/in/a.txt",
        "update_with_include/in/b.txt",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut initial_entries = HashSet::new();
    archive::for_each_entry("update_with_include/archive.pna", |entry| {
        initial_entries.insert(entry.header().path().to_string());
    })
    .unwrap();

    write_newer("update_with_include/in/a.txt", b"new-a");
    write_newer("update_with_include/in/b.txt", b"new-b");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_with_include/archive.pna",
        "update_with_include/in/a.txt",
        "update_with_include/in/b.txt",
        "--keep-timestamp",
        "--include",
        "**/a.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("update_with_include/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert_eq!(
        seen, initial_entries,
        "entry set should be unchanged after update"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "update_with_include/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_with_include/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert_eq!(
        fs::read("update_with_include/out/a.txt").unwrap(),
        b"new-a",
        "included file should be updated"
    );
    assert_eq!(
        fs::read("update_with_include/out/b.txt").unwrap(),
        b"old-b",
        "file outside the include pattern should keep its original content"
    );
}
