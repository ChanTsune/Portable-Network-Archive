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
        "-f",
        "xattr_restore_file/archive.pna",
        "--overwrite",
        "xattr_restore_file/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

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

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
        "xattr_restore_file/archive.pna",
        "--restore",
        "xattr_restore_file/xattrs.dump",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut applied_sorted_by_name =
        archive::xattrs_by_entry("xattr_restore_file/archive.pna", None);
    applied_sorted_by_name.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        applied_sorted_by_name,
        vec![
            (
                "xattr_restore_file/in/raw/empty.txt".to_string(),
                vec![
                    archive::xattr("user.author", b"pna team"),
                    archive::xattr("user.version", b"1.0"),
                ]
            ),
            (
                "xattr_restore_file/in/raw/text.txt".to_string(),
                vec![archive::xattr("user.description", b"sample text file")]
            ),
        ]
    );
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
        "-f",
        "xattr_restore_enc/archive.pna",
        "--overwrite",
        "xattr_restore_enc/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

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
        "-f",
        "xattr_restore_enc/archive.pna",
        "--restore",
        "xattr_restore_enc/xattrs.dump",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::xattrs_by_entry("xattr_restore_enc/archive.pna", None),
        vec![(
            "xattr_restore_enc/in/raw/empty.txt".to_string(),
            vec![
                archive::xattr("user.text", b"hello world"),
                archive::xattr("user.hex", b"HELLO"),
                archive::xattr("user.base64", b"Hello"),
            ]
        )]
    );
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
        "-f",
        "xattr_restore_merge/archive.pna",
        "--overwrite",
        "xattr_restore_merge/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    for (name, value) in [
        ("user.existing", "original"),
        ("user.overwrite", "old_value"),
    ] {
        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "xattr",
            "set",
            "-f",
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
        "-f",
        "xattr_restore_merge/archive.pna",
        "--restore",
        "xattr_restore_merge/xattrs.dump",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::xattrs_by_entry("xattr_restore_merge/archive.pna", None),
        vec![(
            "xattr_restore_merge/in/raw/empty.txt".to_string(),
            vec![
                archive::xattr("user.existing", b"original"),
                archive::xattr("user.overwrite", b"new_value"),
                archive::xattr("user.new", b"added"),
            ]
        )]
    );
}
