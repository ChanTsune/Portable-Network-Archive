use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive with multiple entries exists.
/// Action: Set an extended attribute with `--output` to a new path.
/// Expectation: The output archive has the xattr; the original is untouched.
#[test]
fn xattr_set_output() {
    setup();
    let _ = std::fs::remove_file("xattr_set_output/out.pna");
    TestResources::extract_in("zstd.pna", "xattr_set_output/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
        "xattr_set_output/zstd.pna",
        "--output",
        "xattr_set_output/out.pna",
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
        archive::xattrs_by_entry("xattr_set_output/zstd.pna", None),
        vec![]
    );
    assert_eq!(
        archive::xattrs_by_entry("xattr_set_output/out.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.name", b"pna developers!")]
        )]
    );
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run `pna xattr set` with `--output` pointing at the existing file.
/// Expectation: The command fails without touching either file.
#[test]
fn xattr_set_output_without_overwrite_refuses_to_clobber() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_ow_guard/").unwrap();
    fs::write("xattr_ow_guard/out.pna", b"sentinel").unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
        "xattr_ow_guard/zstd.pna",
        "--output",
        "xattr_ow_guard/out.pna",
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    assert!(format!("{error:?}").contains("already exists"));
    assert_eq!(fs::read("xattr_ow_guard/out.pna").unwrap(), b"sentinel");
    assert_eq!(
        archive::xattrs_by_entry("xattr_ow_guard/zstd.pna", None),
        vec![]
    );
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run the same set with `--overwrite`.
/// Expectation: The output holds the xattr; the original is untouched.
#[test]
fn xattr_set_output_with_overwrite_replaces() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_ow_guard_ok/").unwrap();
    fs::write("xattr_ow_guard_ok/out.pna", b"sentinel").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
        "xattr_ow_guard_ok/zstd.pna",
        "--output",
        "xattr_ow_guard_ok/out.pna",
        "--overwrite",
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
        archive::xattrs_by_entry("xattr_ow_guard_ok/zstd.pna", None),
        vec![]
    );
    assert_eq!(
        archive::xattrs_by_entry("xattr_ow_guard_ok/out.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.name", b"pna developers!")]
        )]
    );
}
