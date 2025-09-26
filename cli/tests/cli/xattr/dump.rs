use crate::utils::{setup, EmbedExt, TestResources};

#[test]
fn xattr_get_dump() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_dump/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "xattr_get_dump/xattr_get_dump.pna",
        "--overwrite",
        "xattr_get_dump/in/",
    ])
    .unwrap();

    let archive_path = "xattr_get_dump/xattr_get_dump.pna";
    let file_to_set_xattr = "xattr_get_dump/in/raw/empty.txt";
    let xattrs_to_set = [
        ("user.meta", "0x0102030405"),
        ("user.name", "pna"),
        ("user.value", "inspired by png data structure"),
    ];

    for (name, value) in xattrs_to_set {
        let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
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
        .unwrap();
    }
    // Sort entries for stablize entries order.
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "xattr_get_dump/xattr_get_dump.pna",
    ])
    .assert();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
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
