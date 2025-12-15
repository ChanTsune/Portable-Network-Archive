use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive exists and a dump file contains xattr definitions.
/// Action: Run `pna xattr set --restore <file>` to restore xattrs from a file path.
/// Expectation: The xattrs defined in the dump file are applied to the archive entries.
#[test]
fn xattr_restore_from_file() {
    setup();
    TestResources::extract_in("raw/", "xattr_restore_file/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_restore_file/archive.pna",
        "--overwrite",
        "xattr_restore_file/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Write the dump content to a file
    fs::write(
        "xattr_restore_file/xattrs.dump",
        concat!(
            "# file: xattr_restore_file/in/raw/empty.txt\n",
            "user.author=\"pna team\"\n",
            "user.version=\"1.0\"\n",
            "\n",
            "# file: xattr_restore_file/in/raw/text.txt\n",
            "user.description=\"sample text file\"\n",
        ),
    )
    .unwrap();

    // Restore xattrs from file
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_restore_file/archive.pna",
        "--restore",
        "xattr_restore_file/xattrs.dump",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify xattrs were restored
    archive::for_each_entry("xattr_restore_file/archive.pna", |entry| {
        match entry.header().path().as_str() {
            "xattr_restore_file/in/raw/empty.txt" => {
                let xattrs = entry.xattrs();
                assert_eq!(xattrs.len(), 2);
                assert!(
                    xattrs
                        .iter()
                        .any(|x| x.name() == "user.author" && x.value() == b"pna team")
                );
                assert!(
                    xattrs
                        .iter()
                        .any(|x| x.name() == "user.version" && x.value() == b"1.0")
                );
            }
            "xattr_restore_file/in/raw/text.txt" => {
                let xattrs = entry.xattrs();
                assert_eq!(xattrs.len(), 1);
                assert_eq!(xattrs[0].name(), "user.description");
                assert_eq!(xattrs[0].value(), b"sample text file");
            }
            _ => {
                assert!(entry.xattrs().is_empty());
            }
        }
    })
    .unwrap();
}

/// Precondition: An archive exists and a dump file contains hex-encoded xattr values.
/// Action: Run `pna xattr set --restore <file>` with hex-encoded values in the dump.
/// Expectation: The binary values are correctly decoded and applied.
#[test]
fn xattr_restore_from_file_with_encodings() {
    setup();
    TestResources::extract_in("raw/", "xattr_restore_enc/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_restore_enc/archive.pna",
        "--overwrite",
        "xattr_restore_enc/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Write dump with different encodings
    fs::write(
        "xattr_restore_enc/xattrs.dump",
        concat!(
            "# file: xattr_restore_enc/in/raw/empty.txt\n",
            "user.text=\"hello world\"\n",
            "user.hex=0x48454c4c4f\n",
            "user.base64=0sSGVsbG8=\n",
        ),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_restore_enc/archive.pna",
        "--restore",
        "xattr_restore_enc/xattrs.dump",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_restore_enc/archive.pna", |entry| {
        if entry.header().path().as_str() == "xattr_restore_enc/in/raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 3);
            assert!(
                xattrs
                    .iter()
                    .any(|x| x.name() == "user.text" && x.value() == b"hello world")
            );
            assert!(
                xattrs
                    .iter()
                    .any(|x| x.name() == "user.hex" && x.value() == b"HELLO")
            );
            assert!(
                xattrs
                    .iter()
                    .any(|x| x.name() == "user.base64" && x.value() == b"Hello")
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive entry already has xattrs, and a dump file defines additional ones.
/// Action: Run `pna xattr set --restore <file>` to add xattrs to an entry with existing xattrs.
/// Expectation: New xattrs are merged with existing ones, overwriting on name collision.
#[test]
fn xattr_restore_from_file_merge() {
    setup();
    TestResources::extract_in("raw/", "xattr_restore_merge/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_restore_merge/archive.pna",
        "--overwrite",
        "xattr_restore_merge/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set initial xattrs
    for (name, value) in [
        ("user.existing", "original"),
        ("user.overwrite", "old_value"),
    ] {
        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "xattr",
            "set",
            "xattr_restore_merge/archive.pna",
            "--name",
            name,
            "--value",
            value,
            "xattr_restore_merge/in/raw/empty.txt",
        ])
        .unwrap()
        .execute()
        .unwrap();
    }

    // Write dump with new and overlapping xattrs
    fs::write(
        "xattr_restore_merge/xattrs.dump",
        concat!(
            "# file: xattr_restore_merge/in/raw/empty.txt\n",
            "user.new=\"added\"\n",
            "user.overwrite=\"new_value\"\n",
        ),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_restore_merge/archive.pna",
        "--restore",
        "xattr_restore_merge/xattrs.dump",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_restore_merge/archive.pna", |entry| {
        if entry.header().path().as_str() == "xattr_restore_merge/in/raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 3, "should have 3 xattrs after merge");
            assert!(
                xattrs
                    .iter()
                    .any(|x| x.name() == "user.existing" && x.value() == b"original"),
                "existing xattr should be preserved"
            );
            assert!(
                xattrs
                    .iter()
                    .any(|x| x.name() == "user.new" && x.value() == b"added"),
                "new xattr should be added"
            );
            assert!(
                xattrs
                    .iter()
                    .any(|x| x.name() == "user.overwrite" && x.value() == b"new_value"),
                "overlapping xattr should be overwritten"
            );
        }
    })
    .unwrap();
}
