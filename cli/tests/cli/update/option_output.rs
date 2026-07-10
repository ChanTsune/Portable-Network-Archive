use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, io::prelude::*, time};

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

/// Precondition: An archive contains multiple files.
/// Action: Run `pna experimental update` with `--output` after modifying one
/// of the source files.
/// Expectation: The source archive is left untouched, and a new archive is
/// written to the given output path reflecting the updated content.
#[test]
fn update_with_output() {
    setup();
    let _ = fs::remove_dir_all("update_with_output");
    fs::create_dir_all("update_with_output/in").unwrap();
    fs::write("update_with_output/in/a.txt", b"old-a").unwrap();
    fs::write("update_with_output/in/b.txt", b"old-b").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_with_output/archive.pna",
        "--overwrite",
        "update_with_output/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let initial_archive = fs::read("update_with_output/archive.pna").unwrap();
    let mut initial_entries = Vec::new();
    archive::for_each_entry("update_with_output/archive.pna", |entry| {
        initial_entries.push(entry.header().path().to_string());
    })
    .unwrap();

    write_newer("update_with_output/in/a.txt", b"new-a");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_with_output/archive.pna",
        "update_with_output/in/",
        "--keep-timestamp",
        "--output",
        "update_with_output/updated.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        fs::read("update_with_output/archive.pna").unwrap(),
        initial_archive,
        "source archive bytes should be unchanged when --output is given"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "update_with_output/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_with_output/src-out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert_eq!(
        fs::read("update_with_output/src-out/a.txt").unwrap(),
        b"old-a",
        "source archive content should remain unchanged"
    );

    let updated_path = "update_with_output/in/a.txt";
    let unchanged_path = "update_with_output/in/b.txt";
    let mut output_entries = Vec::new();
    let mut updated_contents = Vec::new();
    let mut unchanged_contents = Vec::new();
    archive::for_each_entry("update_with_output/updated.pna", |entry| {
        let path = entry.header().path().to_string();
        if path == updated_path || path == unchanged_path {
            let mut content = Vec::new();
            entry
                .reader(pna::ReadOptions::with_password::<&[u8]>(None))
                .unwrap()
                .read_to_end(&mut content)
                .unwrap();
            if path == updated_path {
                updated_contents.push(content);
            } else {
                unchanged_contents.push(content);
            }
        }
        output_entries.push(path);
    })
    .unwrap();

    let mut expected_entries = initial_entries;
    expected_entries.push(updated_path.to_owned());
    assert_eq!(
        output_entries, expected_entries,
        "output archive should preserve the original entry order and append the updated entry"
    );
    assert_eq!(
        updated_contents,
        [b"old-a".to_vec(), b"new-a".to_vec()],
        "updated path should retain the old entry and append the new entry"
    );
    assert_eq!(
        unchanged_contents,
        [b"old-b".to_vec()],
        "unchanged path should occur exactly once"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "update_with_output/updated.pna",
        "--overwrite",
        "--out-dir",
        "update_with_output/out-out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert_eq!(
        fs::read("update_with_output/out-out/a.txt").unwrap(),
        b"new-a",
        "output archive should reflect the updated content"
    );
    assert_eq!(
        fs::read("update_with_output/out-out/b.txt").unwrap(),
        b"old-b",
        "output archive should keep unmodified file content"
    );
}
