use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive entry has multiple extended attributes set.
/// Action: Run `pna xattr get` without `--dump` to list attribute names.
/// Expectation: Only the attribute names are displayed, not their values.
#[test]
fn xattr_get_list_names_only() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_list/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_get_list/archive.pna",
        "--overwrite",
        "xattr_get_list/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set multiple xattrs on the entry
    for (name, value) in [
        ("user.author", "pna developers"),
        ("user.version", "1.0.0"),
        ("user.license", "Apache-2.0"),
    ] {
        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "xattr",
            "set",
            "xattr_get_list/archive.pna",
            "--name",
            name,
            "--value",
            value,
            "xattr_get_list/in/raw/empty.txt",
        ])
        .unwrap()
        .execute()
        .unwrap();
    }

    // Get without --dump should list names only
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "xattr",
            "get",
            "xattr_get_list/archive.pna",
            "xattr_get_list/in/raw/empty.txt",
        ])
        .assert();

    // Output should contain only names, no "=" or values
    assert.stdout(concat!(
        "# file: xattr_get_list/in/raw/empty.txt\n",
        "user.author\n",
        "user.version\n",
        "user.license\n",
        "\n",
    ));
}

/// Precondition: An archive entry has extended attributes set.
/// Action: Run `pna xattr get` without `--dump` but with `--match` to filter names.
/// Expectation: Only matching attribute names are listed, without values.
#[test]
fn xattr_get_list_names_with_match() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_list_match/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_get_list_match/archive.pna",
        "--overwrite",
        "xattr_get_list_match/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set xattrs with different prefixes
    for (name, value) in [
        ("user.name", "test"),
        ("user.email", "test@example.com"),
        ("security.selinux", "context"),
        ("trusted.key", "secret"),
    ] {
        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "xattr",
            "set",
            "xattr_get_list_match/archive.pna",
            "--name",
            name,
            "--value",
            value,
            "xattr_get_list_match/in/raw/empty.txt",
        ])
        .unwrap()
        .execute()
        .unwrap();
    }

    // Get with --match filter, without --dump
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "xattr",
            "get",
            "xattr_get_list_match/archive.pna",
            "xattr_get_list_match/in/raw/empty.txt",
            "--match",
            "^user\\.",
        ])
        .assert();

    // Only user.* attributes should be listed (names only)
    assert.stdout(concat!(
        "# file: xattr_get_list_match/in/raw/empty.txt\n",
        "user.name\n",
        "user.email\n",
        "\n",
    ));
}

/// Precondition: An archive entry has no extended attributes.
/// Action: Run `pna xattr get` without `--dump` on an entry with no xattrs.
/// Expectation: Only the file header is shown with no attribute names.
#[test]
fn xattr_get_list_names_empty() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_list_empty/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_get_list_empty/archive.pna",
        "--overwrite",
        "xattr_get_list_empty/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Get without setting any xattrs
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "xattr",
            "get",
            "xattr_get_list_empty/archive.pna",
            "xattr_get_list_empty/in/raw/empty.txt",
        ])
        .assert();

    // Only file header, no attribute names
    assert.stdout(concat!(
        "# file: xattr_get_list_empty/in/raw/empty.txt\n",
        "\n",
    ));
}
