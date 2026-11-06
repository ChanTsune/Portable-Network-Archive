use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, ReadEntry, ReadOptions};
use predicates::prelude::*;
use std::io::{Cursor, Read as _};

#[test]
fn create_writes_a_complete_archive_to_stdout_while_reading_names_from_stdin() {
    setup();
    let source = "create_stdout/in/raw/text.txt";
    TestResources::extract_in("raw/text.txt", "create_stdout/in/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args(["create", "--files-from-stdin", "--unstable"])
        .write_stdin(format!("{source}\n"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_len = output.stdout.len() as u64;
    let mut archive = Archive::read_header(Cursor::new(output.stdout)).unwrap();
    {
        let mut entries = archive.entries();
        let ReadEntry::Normal(entry) = entries.next().unwrap().unwrap() else {
            panic!("ordinary create unexpectedly emitted a solid entry");
        };
        assert_eq!(entry.name(), source);
        let mut content = Vec::new();
        entry
            .reader(ReadOptions::builder().build())
            .unwrap()
            .read_to_end(&mut content)
            .unwrap();
        assert_eq!(
            content.as_slice(),
            TestResources::get("raw/text.txt").unwrap().data.as_ref()
        );
        assert!(entries.next().is_none());
    }
    assert_eq!(archive.into_inner().position(), output_len);
}

#[test]
fn create_rejects_overwrite_without_a_file_destination() {
    setup();

    cargo_bin_cmd!("pna")
        .args(["create", "--overwrite"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "--overwrite requires --file PATH when creating an archive",
        ));
}
