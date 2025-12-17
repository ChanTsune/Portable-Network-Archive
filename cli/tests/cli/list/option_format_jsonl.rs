use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list --format jsonl`.
/// Expectation: Each entry is output as a valid JSON line with required fields.
#[test]
fn list_format_jsonl() {
    setup();
    TestResources::extract_in("raw/", "list_format_jsonl/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_jsonl/archive.pna",
        "--overwrite",
        "list_format_jsonl/in/",
    ])
    .assert()
    .success();

    // Sort entries for stable order
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "list_format_jsonl/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "jsonl",
            "list_format_jsonl/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        r#"{"filename":"list_format_jsonl/in/raw/empty.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":9,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl/in/raw/first/second/third/pna.txt","permissions":"---------- ","owner":"","group":"","raw_size":3,"size":12,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl/in/raw/images/icon.bmp","permissions":"---------- ","owner":"","group":"","raw_size":4194442,"size":17183,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl/in/raw/images/icon.png","permissions":"---------- ","owner":"","group":"","raw_size":51475,"size":38437,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl/in/raw/images/icon.svg","permissions":"---------- ","owner":"","group":"","raw_size":1984,"size":788,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl/in/raw/parent/child.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":9,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl/in/raw/pna/empty.pna","permissions":"---------- ","owner":"","group":"","raw_size":40,"size":49,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl/in/raw/pna/nest.pna","permissions":"---------- ","owner":"","group":"","raw_size":57032,"size":57041,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl/in/raw/text.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":19,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
    ));
}

/// Precondition: A solid archive contains multiple file entries.
/// Action: Run `pna list --format jsonl --solid`.
/// Expectation: Solid entries are output as valid JSON lines.
#[test]
fn list_format_jsonl_solid() {
    setup();
    TestResources::extract_in("raw/", "list_format_jsonl_solid/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_jsonl_solid/archive.pna",
        "--overwrite",
        "--solid",
        "list_format_jsonl_solid/in/",
    ])
    .assert()
    .success();

    // Sort entries for stable order
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "list_format_jsonl_solid/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "jsonl",
            "--solid",
            "list_format_jsonl_solid/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        r#"{"filename":"list_format_jsonl_solid/in/raw/empty.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":0,"encryption":"-","compression":"-","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_solid/in/raw/first/second/third/pna.txt","permissions":"---------- ","owner":"","group":"","raw_size":3,"size":3,"encryption":"-","compression":"-","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_solid/in/raw/images/icon.bmp","permissions":"---------- ","owner":"","group":"","raw_size":4194442,"size":4194442,"encryption":"-","compression":"-","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_solid/in/raw/images/icon.png","permissions":"---------- ","owner":"","group":"","raw_size":51475,"size":51475,"encryption":"-","compression":"-","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_solid/in/raw/images/icon.svg","permissions":"---------- ","owner":"","group":"","raw_size":1984,"size":1984,"encryption":"-","compression":"-","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_solid/in/raw/parent/child.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":0,"encryption":"-","compression":"-","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_solid/in/raw/pna/empty.pna","permissions":"---------- ","owner":"","group":"","raw_size":40,"size":40,"encryption":"-","compression":"-","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_solid/in/raw/pna/nest.pna","permissions":"---------- ","owner":"","group":"","raw_size":57032,"size":57032,"encryption":"-","compression":"-","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_solid/in/raw/text.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":10,"encryption":"-","compression":"-","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
    ));
}

/// Precondition: An encrypted archive contains file entries.
/// Action: Run `pna list --format jsonl` with password.
/// Expectation: Entries show encryption type in JSON output.
#[test]
fn list_format_jsonl_encrypted() {
    setup();
    TestResources::extract_in("raw/", "list_format_jsonl_enc/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_jsonl_enc/archive.pna",
        "--overwrite",
        "list_format_jsonl_enc/in/",
        "--password",
        "testpassword",
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
    ])
    .assert()
    .success();

    // Sort entries for stable order
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "list_format_jsonl_enc/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "jsonl",
            "list_format_jsonl_enc/archive.pna",
            "--password",
            "testpassword",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        r#"{"filename":"list_format_jsonl_enc/in/raw/empty.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":25,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_enc/in/raw/first/second/third/pna.txt","permissions":"---------- ","owner":"","group":"","raw_size":3,"size":28,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_enc/in/raw/images/icon.bmp","permissions":"---------- ","owner":"","group":"","raw_size":4194442,"size":17199,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_enc/in/raw/images/icon.png","permissions":"---------- ","owner":"","group":"","raw_size":51475,"size":38453,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_enc/in/raw/images/icon.svg","permissions":"---------- ","owner":"","group":"","raw_size":1984,"size":804,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_enc/in/raw/parent/child.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":25,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_enc/in/raw/pna/empty.pna","permissions":"---------- ","owner":"","group":"","raw_size":40,"size":65,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_enc/in/raw/pna/nest.pna","permissions":"---------- ","owner":"","group":"","raw_size":57032,"size":57057,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"list_format_jsonl_enc/in/raw/text.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":35,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
    ));
}
