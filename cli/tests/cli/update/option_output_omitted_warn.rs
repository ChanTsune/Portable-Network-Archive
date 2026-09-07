#![cfg(not(target_family = "wasm"))]
use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;
use std::{fs, io::prelude::*, path::Path, time};

/// Precondition: An archive is created from a source tree, then one source
/// file is modified with a newer mtime.
/// Action: Run `experimental update` without `--output` (no `--quiet`, so
/// warnings stay visible).
/// Expectation: Succeeds, the archive is rewritten in place with the updated
/// content, and a deprecation warning names the rewritten path.
#[test]
fn update_without_output_warns_but_rewrites_in_place() {
    setup();
    let _ = fs::remove_dir_all("update_omitted_output_warn");
    fs::create_dir_all("update_omitted_output_warn/in").unwrap();
    fs::write("update_omitted_output_warn/in/a.txt", b"old-a").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "update_omitted_output_warn/archive.pna",
        "--overwrite",
        "update_omitted_output_warn/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_omitted_output_warn/in/a.txt")
        .unwrap();
    file.write_all(b"new-a").unwrap();
    file.set_modified(time::SystemTime::now() + time::Duration::from_secs(24 * 60 * 60))
        .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "update",
            "-f",
            "update_omitted_output_warn/archive.pna",
            "update_omitted_output_warn/in/",
            "--keep-timestamp",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "omitting `--output` is deprecated and will write to standard output instead of rewriting",
        ))
        .stderr(predicate::str::contains(
            Path::new("update_omitted_output_warn/archive.pna")
                .display()
                .to_string(),
        ));

    let mut updated_contents = Vec::new();
    archive::for_each_entry("update_omitted_output_warn/archive.pna", |entry| {
        if entry.header().path().as_str() == "update_omitted_output_warn/in/a.txt" {
            let mut content = Vec::new();
            entry
                .reader(pna::ReadOptions::with_password::<&[u8]>(None))
                .unwrap()
                .read_to_end(&mut content)
                .unwrap();
            updated_contents.push(content);
        }
    })
    .unwrap();
    assert_eq!(
        updated_contents,
        [b"old-a".to_vec(), b"new-a".to_vec()],
        "updated path should retain the old entry and append the new entry"
    );
}
