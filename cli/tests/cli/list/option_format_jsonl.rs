use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list --format jsonl`.
/// Expectation: Each entry is output as a valid JSON line with required fields.
#[test]
fn list_format_jsonl() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_format_jsonl/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "jsonl",
            "-f",
            "list_format_jsonl/zstd_with_raw_file_size.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        r#"{"filename":"raw/images/icon.png","permissions":"---------- ","owner":"","group":"","raw_size":51475,"size":38437,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/empty.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":9,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/images/icon.svg","permissions":"---------- ","owner":"","group":"","raw_size":1984,"size":789,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/first/second/third/pna.txt","permissions":"---------- ","owner":"","group":"","raw_size":3,"size":12,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/pna/empty.pna","permissions":"---------- ","owner":"","group":"","raw_size":40,"size":49,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/parent/child.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":9,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/pna/nest.pna","permissions":"---------- ","owner":"","group":"","raw_size":57032,"size":57041,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/text.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":19,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/images/icon.bmp","permissions":"---------- ","owner":"","group":"","raw_size":4194442,"size":17183,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
    ));
}

/// Precondition: A solid archive contains multiple file entries.
/// Action: Run `pna list --format jsonl --solid`.
/// Expectation: Solid entries are output as valid JSON lines.
#[test]
fn list_format_jsonl_solid() {
    setup();
    TestResources::extract_in("solid_zstd.pna", "list_format_jsonl_solid/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "jsonl",
            "--solid",
            "-f",
            "list_format_jsonl_solid/solid_zstd.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        r#"{"filename":"raw/empty.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":0,"encryption":"-","compression":"zstandard(solid)","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/parent/child.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":0,"encryption":"-","compression":"zstandard(solid)","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/images/icon.svg","permissions":"---------- ","owner":"","group":"","raw_size":1984,"size":1984,"encryption":"-","compression":"zstandard(solid)","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/first/second/third/pna.txt","permissions":"---------- ","owner":"","group":"","raw_size":3,"size":3,"encryption":"-","compression":"zstandard(solid)","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/images/icon.png","permissions":"---------- ","owner":"","group":"","raw_size":51475,"size":51475,"encryption":"-","compression":"zstandard(solid)","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/pna/nest.pna","permissions":"---------- ","owner":"","group":"","raw_size":57032,"size":57032,"encryption":"-","compression":"zstandard(solid)","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/text.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":10,"encryption":"-","compression":"zstandard(solid)","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/pna/empty.pna","permissions":"---------- ","owner":"","group":"","raw_size":40,"size":40,"encryption":"-","compression":"zstandard(solid)","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/images/icon.bmp","permissions":"---------- ","owner":"","group":"","raw_size":4194442,"size":4194442,"encryption":"-","compression":"zstandard(solid)","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
    ));
}

/// Precondition: An encrypted archive contains file entries.
/// Action: Run `pna list --format jsonl` with password.
/// Expectation: Entries show encryption type in JSON output.
#[test]
fn list_format_jsonl_encrypted() {
    setup();
    TestResources::extract_in("zstd_aes_ctr.pna", "list_format_jsonl_enc/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "jsonl",
            "-f",
            "list_format_jsonl_enc/zstd_aes_ctr.pna",
            "--password",
            "password",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        r#"{"filename":"raw/empty.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":25,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/images/icon.png","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":35142,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/images/icon.svg","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":751,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/images/icon.bmp","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":13378,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/first/second/third/pna.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":28,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/pna/empty.pna","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":65,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/pna/nest.pna","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":53757,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/parent/child.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":25,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
        r#"{"filename":"raw/text.txt","permissions":"---------- ","owner":"","group":"","raw_size":0,"size":35,"encryption":"aes(ctr)","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
        "\n",
    ));
}
