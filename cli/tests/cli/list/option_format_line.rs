use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list` with default format (no --format option).
/// Expectation: Each entry path is output on a separate line.
#[test]
fn list_default_format() {
    setup();
    TestResources::extract_in("raw/", "list_default_format/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_default_format/archive.pna",
        "--overwrite",
        "list_default_format/in/",
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
        "list_default_format/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args(["list", "list_default_format/archive.pna"])
        .assert();

    assert.stdout(concat!(
        "list_default_format/in/raw/empty.txt\n",
        "list_default_format/in/raw/first/second/third/pna.txt\n",
        "list_default_format/in/raw/images/icon.bmp\n",
        "list_default_format/in/raw/images/icon.png\n",
        "list_default_format/in/raw/images/icon.svg\n",
        "list_default_format/in/raw/parent/child.txt\n",
        "list_default_format/in/raw/pna/empty.pna\n",
        "list_default_format/in/raw/pna/nest.pna\n",
        "list_default_format/in/raw/text.txt\n",
    ));
}

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list --format line` explicitly.
/// Expectation: Output is identical to default format - each entry path on a separate line.
#[test]
fn list_format_line() {
    setup();
    TestResources::extract_in("raw/", "list_format_line/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_line/archive.pna",
        "--overwrite",
        "list_format_line/in/",
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
        "list_format_line/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "line",
            "list_format_line/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "list_format_line/in/raw/empty.txt\n",
        "list_format_line/in/raw/first/second/third/pna.txt\n",
        "list_format_line/in/raw/images/icon.bmp\n",
        "list_format_line/in/raw/images/icon.png\n",
        "list_format_line/in/raw/images/icon.svg\n",
        "list_format_line/in/raw/parent/child.txt\n",
        "list_format_line/in/raw/pna/empty.pna\n",
        "list_format_line/in/raw/pna/nest.pna\n",
        "list_format_line/in/raw/text.txt\n",
    ));
}

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list` default format and `pna list --format line` on the same archive.
/// Expectation: Both commands produce identical output.
#[test]
fn list_default_and_format_line_are_identical() {
    setup();
    TestResources::extract_in("raw/", "list_identical/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_identical/archive.pna",
        "--overwrite",
        "list_identical/in/",
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
        "list_identical/archive.pna",
    ])
    .assert()
    .success();

    // Get default format output
    let mut cmd = cargo_bin_cmd!("pna");
    let default_output = cmd
        .args(["list", "list_identical/archive.pna"])
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
            "list_identical/archive.pna",
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
