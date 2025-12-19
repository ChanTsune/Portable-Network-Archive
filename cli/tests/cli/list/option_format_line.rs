use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: A solid archive contains multiple file entries.
/// Action: Run `pna list --solid` with default format.
/// Expectation: All solid entries are listed, one per line.
#[test]
fn list_solid() {
    setup();
    TestResources::extract_in("solid_zstd.pna", "").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args(["list", "-f", "solid_zstd.pna", "--solid"])
        .assert();

    assert.stdout(concat!(
        "raw/empty.txt\n",
        "raw/parent/child.txt\n",
        "raw/images/icon.svg\n",
        "raw/first/second/third/pna.txt\n",
        "raw/images/icon.png\n",
        "raw/pna/nest.pna\n",
        "raw/text.txt\n",
        "raw/pna/empty.pna\n",
        "raw/images/icon.bmp\n",
    ));
}

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list` with default format (no --format option).
/// Expectation: Each entry path is output on a separate line.
#[test]
fn list_default_format() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_default_format/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "-f",
            "list_default_format/zstd_with_raw_file_size.pna",
        ])
        .assert();

    assert.stdout(concat!(
        "raw/images/icon.png\n",
        "raw/empty.txt\n",
        "raw/images/icon.svg\n",
        "raw/first/second/third/pna.txt\n",
        "raw/pna/empty.pna\n",
        "raw/parent/child.txt\n",
        "raw/pna/nest.pna\n",
        "raw/text.txt\n",
        "raw/images/icon.bmp\n",
    ));
}

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list --format line` explicitly.
/// Expectation: Output is identical to default format - each entry path on a separate line.
#[test]
fn list_format_line() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_format_line/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "line",
            "-f",
            "list_format_line/zstd_with_raw_file_size.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "raw/images/icon.png\n",
        "raw/empty.txt\n",
        "raw/images/icon.svg\n",
        "raw/first/second/third/pna.txt\n",
        "raw/pna/empty.pna\n",
        "raw/parent/child.txt\n",
        "raw/pna/nest.pna\n",
        "raw/text.txt\n",
        "raw/images/icon.bmp\n",
    ));
}

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list` default format and `pna list --format line` on the same archive.
/// Expectation: Both commands produce identical output.
#[test]
fn list_default_and_format_line_are_identical() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_identical/").unwrap();

    // Get default format output
    let mut cmd = cargo_bin_cmd!("pna");
    let default_output = cmd
        .args(["list", "-f", "list_identical/zstd_with_raw_file_size.pna"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Get --format line output
    let mut cmd = cargo_bin_cmd!("pna");
    let line_output = cmd
        .args([
            "list",
            "--format",
            "line",
            "-f",
            "list_identical/zstd_with_raw_file_size.pna",
            "--unstable",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_eq!(
        default_output, line_output,
        "default format and --format line should produce identical output"
    );
}
