use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::{fs, io::Read};

/// Precondition: An archive contains an entry and an additional input file exists.
/// Action: Append with explicit in-place intent.
/// Expectation: The input archive contains both entries and stdout and stderr are empty.
#[test]
fn append_with_overwrite_updates_existing_archive() {
    setup();
    fs::create_dir_all("append_overwrite").unwrap();
    archive::create_archive_with_permissions(
        "append_overwrite/base.pna",
        &[archive::FileEntryDef {
            path: "base.txt",
            content: b"base content",
            permission: 0o644,
        }],
    )
    .unwrap();
    fs::write("append_overwrite/new.txt", b"new content").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--log-level",
            "warn",
            "append",
            "--overwrite",
            "--file",
            "append_overwrite/base.pna",
            "append_overwrite/new.txt",
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let mut entries = Vec::new();
    archive::for_each_entry("append_overwrite/base.pna", |entry| {
        let mut content = Vec::new();
        entry
            .reader(pna::ReadOptions::with_password::<&[u8]>(None))
            .unwrap()
            .read_to_end(&mut content)
            .unwrap();
        entries.push((entry.header().path().to_string(), content));
    })
    .unwrap();
    assert_eq!(
        entries,
        [
            ("base.txt".to_string(), b"base content".to_vec()),
            (
                "append_overwrite/new.txt".to_string(),
                b"new content".to_vec()
            ),
        ]
    );
}
