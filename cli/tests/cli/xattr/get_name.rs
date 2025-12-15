use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive entry has multiple extended attributes set.
/// Action: Run `pna xattr get` with `--name` to retrieve a specific attribute by name.
/// Expectation: Only the named attribute is displayed with its value (--name implies --dump).
#[test]
fn xattr_get_by_name() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_name/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_get_name/archive.pna",
        "--overwrite",
        "xattr_get_name/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set multiple xattrs on the same entry
    for (name, value) in [
        ("user.first", "value1"),
        ("user.second", "value2"),
        ("user.third", "value3"),
    ] {
        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "xattr",
            "set",
            "xattr_get_name/archive.pna",
            "--name",
            name,
            "--value",
            value,
            "xattr_get_name/in/raw/empty.txt",
        ])
        .unwrap()
        .execute()
        .unwrap();
    }

    // Get only the specific attribute by name
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "xattr",
            "get",
            "xattr_get_name/archive.pna",
            "xattr_get_name/in/raw/empty.txt",
            "--name",
            "user.second",
        ])
        .assert();

    // --name implies --dump, so the value should be shown
    // Only the named attribute should appear in output
    assert.stdout(concat!(
        "# file: xattr_get_name/in/raw/empty.txt\n",
        "user.second=\"value2\"\n",
        "\n",
    ));
}

/// Precondition: An archive entry has an extended attribute with binary data.
/// Action: Run `pna xattr get` with `--name` and `--encoding hex` to retrieve it.
/// Expectation: The attribute value is displayed in hex encoding.
#[test]
fn xattr_get_by_name_with_encoding() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_name_enc/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_get_name_enc/archive.pna",
        "--overwrite",
        "xattr_get_name_enc/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set xattr with binary data using hex encoding
    for (name, value) in [("user.binary", "0x00010203"), ("user.text", "hello")] {
        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "xattr",
            "set",
            "xattr_get_name_enc/archive.pna",
            "--name",
            name,
            "--value",
            value,
            "xattr_get_name_enc/in/raw/empty.txt",
        ])
        .unwrap()
        .execute()
        .unwrap();
    }

    // Get specific attribute with hex encoding
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "xattr",
            "get",
            "xattr_get_name_enc/archive.pna",
            "xattr_get_name_enc/in/raw/empty.txt",
            "--name",
            "user.binary",
            "--encoding",
            "hex",
        ])
        .assert();

    assert.stdout(concat!(
        "# file: xattr_get_name_enc/in/raw/empty.txt\n",
        "user.binary=0x00010203\n",
        "\n",
    ));
}

/// Precondition: An archive entry has no extended attribute with the requested name.
/// Action: Run `pna xattr get` with `--name` for a non-existent attribute.
/// Expectation: The output shows the file header but no attribute line.
#[test]
fn xattr_get_by_name_not_found() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_name_notfound/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_get_name_notfound/archive.pna",
        "--overwrite",
        "xattr_get_name_notfound/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set an xattr with a different name
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_get_name_notfound/archive.pna",
        "--name",
        "user.exists",
        "--value",
        "data",
        "xattr_get_name_notfound/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Try to get a non-existent attribute
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "xattr",
            "get",
            "xattr_get_name_notfound/archive.pna",
            "xattr_get_name_notfound/in/raw/empty.txt",
            "--name",
            "user.notfound",
        ])
        .assert();

    // File header should appear, but no attribute line since no match
    assert.stdout(concat!(
        "# file: xattr_get_name_notfound/in/raw/empty.txt\n",
        "\n",
    ));
}
