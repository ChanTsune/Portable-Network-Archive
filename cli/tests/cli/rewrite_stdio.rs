use crate::utils::TestResources;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::Archive;
use std::io::Cursor;

#[test]
fn ordinary_rewrite_family_accepts_stdin_and_emits_a_complete_archive() {
    let archive = TestResources::get("zstd.pna").unwrap().data.into_owned();
    let cases: &[&[&str]] = &[
        &["delete", "raw/empty.txt"],
        &["experimental", "chmod", "600", "raw/text.txt"],
        &[
            "experimental",
            "chown",
            "new_user",
            "raw/text.txt",
            "--no-owner-lookup",
        ],
        &[
            "experimental",
            "acl",
            "set",
            "raw/text.txt",
            "--modify",
            "u:test:r",
        ],
        &[
            "xattr",
            "set",
            "raw/text.txt",
            "--name",
            "user.stage12",
            "--value",
            "value",
        ],
        &["migrate"],
    ];

    for args in cases {
        let output = cargo_bin_cmd!("pna")
            .args(*args)
            .write_stdin(archive.clone())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let mut rewritten = Archive::read_header(Cursor::new(output.stdout)).unwrap();
        let entries = rewritten
            .entries()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        assert!(!entries.is_empty(), "{args:?} emitted an empty archive");
    }
}
