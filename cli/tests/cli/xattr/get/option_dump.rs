use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive entry has multiple extended attributes set.
/// Action: Run `pna xattr get` with `--dump` to display attribute values.
/// Expectation: All attributes are shown with their values in quoted format.
#[test]
fn xattr_get_dump() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_dump/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "xattr_get_dump/xattr_get_dump.pna",
        "--overwrite",
        "xattr_get_dump/in/",
    ])
    .assert()
    .success();

    let archive_path = "xattr_get_dump/xattr_get_dump.pna";
    let file_to_set_xattr = "xattr_get_dump/in/raw/empty.txt";
    let xattrs_to_set = [
        ("user.meta", "0x0102030405"),
        ("user.name", "pna"),
        ("user.value", "inspired by png data structure"),
    ];

    for (name, value) in xattrs_to_set {
        let mut cmd = cargo_bin_cmd!("pna");
        cmd.args([
            "--quiet",
            "xattr",
            "set",
            archive_path,
            file_to_set_xattr,
            "--name",
            name,
            "--value",
            value,
        ])
        .assert()
        .success();
    }
    // Sort entries to stabilize entry order.
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "xattr_get_dump/xattr_get_dump.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "xattr",
            "get",
            "xattr_get_dump/xattr_get_dump.pna",
            "xattr_get_dump/in/raw/empty.txt",
            "--dump",
        ])
        .assert();

    assert.stdout(concat!(
        "# file: xattr_get_dump/in/raw/empty.txt\n",
        "user.meta=\"\x01\x02\x03\x04\x05\"\n",
        "user.name=\"pna\"\n",
        "user.value=\"inspired by png data structure\"\n",
        "\n",
    ));
}
