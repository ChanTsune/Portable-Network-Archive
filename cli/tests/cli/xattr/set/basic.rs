use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive with multiple entries exists.
/// Action: Set an extended attribute on a specific entry.
/// Expectation: Target entry has the xattr; other entries remain unaffected.
#[test]
fn archive_xattr_set() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_set/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_set/zstd.pna",
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_set/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.name".into(),
                    b"pna developers!".into()
                )]
            );
        } else {
            // Non-target entries should remain unaffected (no xattrs)
            assert!(
                entry.xattrs().is_empty(),
                "Entry {} should have no xattrs but has {:?}",
                entry.name(),
                entry.xattrs()
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive with multiple entries exists.
/// Action: Set xattrs with long name (200+ chars), long value (1024 bytes), and special characters.
/// Expectation: Target entry has the xattrs; other entries remain unaffected.
#[test]
fn xattr_long_key_value() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_long/").unwrap();

    let long_name = "user.".to_owned() + &"n".repeat(200);
    let long_value = "v".repeat(1024);
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_long/zstd.pna",
        "--name",
        &long_name,
        "--value",
        &long_value,
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_long/zstd.pna",
        "--name",
        "user.special",
        "--value",
        "\0\n\r\x7f\u{1F600}",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_long/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[
                    pna::ExtendedAttribute::new(
                        long_name.as_str().into(),
                        long_value.as_bytes().into()
                    ),
                    pna::ExtendedAttribute::new(
                        "user.special".into(),
                        "\0\n\r\x7f\u{1F600}".into()
                    ),
                ]
            );
        } else {
            // Non-target entries should remain unaffected (no xattrs)
            assert!(
                entry.xattrs().is_empty(),
                "Entry {} should have no xattrs but has {:?}",
                entry.name(),
                entry.xattrs()
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive with multiple entries exists.
/// Action: Set an xattr with an empty key name.
/// Expectation: Target entry has the xattr with empty key; other entries remain unaffected.
#[test]
fn xattr_empty_key() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_empty_key/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_empty_key/zstd.pna",
        "--name",
        "",
        "--value",
        "value",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_empty_key/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new("".into(), b"value".into())]
            );
        } else {
            // Non-target entries should remain unaffected (no xattrs)
            assert!(
                entry.xattrs().is_empty(),
                "Entry {} should have no xattrs but has {:?}",
                entry.name(),
                entry.xattrs()
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive with multiple entries exists.
/// Action: Set an xattr with an empty value.
/// Expectation: Target entry has the xattr with empty value; other entries remain unaffected.
#[test]
fn xattr_empty_value() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_empty_value/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_empty_value/zstd.pna",
        "--name",
        "user.empty",
        "--value",
        "",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_empty_value/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new("user.empty".into(), b"".into())]
            );
        } else {
            // Non-target entries should remain unaffected (no xattrs)
            assert!(
                entry.xattrs().is_empty(),
                "Entry {} should have no xattrs but has {:?}",
                entry.name(),
                entry.xattrs()
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive with multiple entries exists.
/// Action: Set an xattr on a specific entry via `pna xattr set`.
/// Expectation: The xattr data is correctly stored in the archive entry.
#[test]
fn xattr_set_preserved_in_archive() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_set_preserved/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_set_preserved/zstd.pna",
        "--name",
        "user.roundtrip",
        "--value",
        "preserved_value",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    archive::for_each_entry("xattr_set_preserved/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            found = true;
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.roundtrip".into(),
                    b"preserved_value".into()
                )]
            );
        }
    })
    .unwrap();
    assert!(found, "raw/empty.txt entry not found in archive");
}

/// Precondition: An archive entry has extended attributes set.
/// Action: Extract with `--keep-xattr`, then re-create from extracted files with `--keep-xattr`.
/// Expectation: The xattr data in the new archive matches the original.
#[test]
#[ignore]
fn xattr_round_trip_preservation() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_roundtrip/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_roundtrip/zstd.pna",
        "--name",
        "user.roundtrip",
        "--value",
        "preserved_value",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "xattr_roundtrip/zstd.pna",
        "--overwrite",
        "--out-dir",
        "xattr_roundtrip/out/",
        "--keep-xattr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_roundtrip/roundtrip.pna",
        "--overwrite",
        "xattr_roundtrip/out/",
        "--keep-xattr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    archive::for_each_entry("xattr_roundtrip/roundtrip.pna", |entry| {
        if entry.name().as_str().ends_with("raw/empty.txt") {
            found = true;
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.roundtrip".into(),
                    b"preserved_value".into()
                )],
            );
        }
    })
    .unwrap();
    assert!(found, "raw/empty.txt entry not found in round-trip archive");
}
