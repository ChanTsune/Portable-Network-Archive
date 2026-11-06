#[cfg(unix)]
use crate::utils::fs_supports_xattr;
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
        "--overwrite",
        "-f",
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

    assert_eq!(
        archive::xattrs_by_entry("xattr_set/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.name", b"pna developers!")]
        )]
    );
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
        "--overwrite",
        "-f",
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
        "--overwrite",
        "-f",
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

    assert_eq!(
        archive::xattrs_by_entry("xattr_long/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![
                archive::xattr(long_name.as_str(), long_value.as_bytes()),
                archive::xattr("user.special", "\0\n\r\x7f\u{1F600}".as_bytes()),
            ]
        )]
    );
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
        "--overwrite",
        "-f",
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

    assert_eq!(
        archive::xattrs_by_entry("xattr_empty_key/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("", b"value")]
        )]
    );
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
        "--overwrite",
        "-f",
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

    assert_eq!(
        archive::xattrs_by_entry("xattr_empty_value/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.empty", b"")]
        )]
    );
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
        "--overwrite",
        "-f",
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

    assert_eq!(
        archive::xattrs_by_entry("xattr_set_preserved/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.roundtrip", b"preserved_value")]
        )]
    );
}

/// Precondition: An archive entry has extended attributes set.
/// Action: Extract with `--keep-xattr`, then re-create from extracted files with `--keep-xattr`.
/// Expectation: The xattr data in the new archive matches the original.
#[test]
#[cfg(unix)]
fn xattr_round_trip_preservation() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_roundtrip/").unwrap();
    skip_unless!("xattr", fs_supports_xattr("xattr_roundtrip/zstd.pna"));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "--overwrite",
        "-f",
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
        "-f",
        "xattr_roundtrip/zstd.pna",
        "--overwrite",
        "--out-dir",
        "xattr_roundtrip/out/",
        "--keep-xattr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        xattr::get("xattr_roundtrip/out/raw/empty.txt", "user.roundtrip")
            .unwrap()
            .as_deref(),
        Some(b"preserved_value".as_slice())
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "xattr_roundtrip/roundtrip.pna",
        "--overwrite",
        "xattr_roundtrip/out/",
        "--keep-xattr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut xattrs_by_entry = archive::xattrs_by_entry("xattr_roundtrip/roundtrip.pna", None);
    for (_, xattrs) in &mut xattrs_by_entry {
        xattrs.retain(|x| x.name() != "com.apple.provenance");
    }
    xattrs_by_entry.retain(|(_, xattrs)| !xattrs.is_empty());

    assert_eq!(
        xattrs_by_entry,
        vec![(
            "xattr_roundtrip/out/raw/empty.txt".to_string(),
            vec![archive::xattr("user.roundtrip", b"preserved_value")]
        )]
    );
}
