use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains multiple file entries with preserved timestamps.
/// Action: Run `pna list --format bsdtar`.
/// Expectation: Entries are displayed in bsdtar format with permissions, size, timestamp, and filename.
#[test]
fn list_format_bsdtar() {
    setup();
    TestResources::extract_in("raw/", "list_format_bsdtar/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_bsdtar/archive.pna",
        "--overwrite",
        "--keep-timestamp",
        "--mtime",
        "2023-01-01 00:00:00",
        "list_format_bsdtar/in/",
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
        "list_format_bsdtar/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "bsdtar",
            "list_format_bsdtar/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "----------   0                      0 Jan  1  2023 list_format_bsdtar/in/raw/empty.txt\n",
        "----------   0                      3 Jan  1  2023 list_format_bsdtar/in/raw/first/second/third/pna.txt\n",
        "----------   0                4194442 Jan  1  2023 list_format_bsdtar/in/raw/images/icon.bmp\n",
        "----------   0                  51475 Jan  1  2023 list_format_bsdtar/in/raw/images/icon.png\n",
        "----------   0                   1984 Jan  1  2023 list_format_bsdtar/in/raw/images/icon.svg\n",
        "----------   0                      0 Jan  1  2023 list_format_bsdtar/in/raw/parent/child.txt\n",
        "----------   0                     40 Jan  1  2023 list_format_bsdtar/in/raw/pna/empty.pna\n",
        "----------   0                  57032 Jan  1  2023 list_format_bsdtar/in/raw/pna/nest.pna\n",
        "----------   0                     10 Jan  1  2023 list_format_bsdtar/in/raw/text.txt\n",
    ));
}

/// Precondition: A solid archive contains multiple file entries with preserved timestamps.
/// Action: Run `pna list --format bsdtar --solid`.
/// Expectation: Solid entries are displayed in bsdtar format.
#[test]
fn list_format_bsdtar_solid() {
    setup();
    TestResources::extract_in("raw/", "list_format_bsdtar_solid/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_bsdtar_solid/archive.pna",
        "--overwrite",
        "--solid",
        "--keep-timestamp",
        "--mtime",
        "2023-01-01 00:00:00",
        "list_format_bsdtar_solid/in/",
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
        "list_format_bsdtar_solid/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "bsdtar",
            "--solid",
            "list_format_bsdtar_solid/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "----------   0                      0 Jan  1  2023 list_format_bsdtar_solid/in/raw/empty.txt\n",
        "----------   0                      3 Jan  1  2023 list_format_bsdtar_solid/in/raw/first/second/third/pna.txt\n",
        "----------   0                4194442 Jan  1  2023 list_format_bsdtar_solid/in/raw/images/icon.bmp\n",
        "----------   0                  51475 Jan  1  2023 list_format_bsdtar_solid/in/raw/images/icon.png\n",
        "----------   0                   1984 Jan  1  2023 list_format_bsdtar_solid/in/raw/images/icon.svg\n",
        "----------   0                      0 Jan  1  2023 list_format_bsdtar_solid/in/raw/parent/child.txt\n",
        "----------   0                     40 Jan  1  2023 list_format_bsdtar_solid/in/raw/pna/empty.pna\n",
        "----------   0                  57032 Jan  1  2023 list_format_bsdtar_solid/in/raw/pna/nest.pna\n",
        "----------   0                     10 Jan  1  2023 list_format_bsdtar_solid/in/raw/text.txt\n",
    ));
}

/// Precondition: An archive contains directories and files with --keep-dir.
/// Action: Run `pna list --format bsdtar`.
/// Expectation: Directories are displayed with 'd' permission prefix.
#[test]
fn list_format_bsdtar_with_directories() {
    setup();
    TestResources::extract_in("raw/", "list_bsdtar_dir/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_bsdtar_dir/archive.pna",
        "--overwrite",
        "--keep-dir",
        "--keep-timestamp",
        "--mtime",
        "2023-01-01 00:00:00",
        "list_bsdtar_dir/in/",
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
        "list_bsdtar_dir/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "bsdtar",
            "list_bsdtar_dir/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "d---------   0                      0 Jan  1  2023 list_bsdtar_dir/in/\n",
        "d---------   0                      0 Jan  1  2023 list_bsdtar_dir/in/raw/\n",
        "----------   0                      0 Jan  1  2023 list_bsdtar_dir/in/raw/empty.txt\n",
        "d---------   0                      0 Jan  1  2023 list_bsdtar_dir/in/raw/first/\n",
        "d---------   0                      0 Jan  1  2023 list_bsdtar_dir/in/raw/first/second/\n",
        "d---------   0                      0 Jan  1  2023 list_bsdtar_dir/in/raw/first/second/third/\n",
        "----------   0                      3 Jan  1  2023 list_bsdtar_dir/in/raw/first/second/third/pna.txt\n",
        "d---------   0                      0 Jan  1  2023 list_bsdtar_dir/in/raw/images/\n",
        "----------   0                4194442 Jan  1  2023 list_bsdtar_dir/in/raw/images/icon.bmp\n",
        "----------   0                  51475 Jan  1  2023 list_bsdtar_dir/in/raw/images/icon.png\n",
        "----------   0                   1984 Jan  1  2023 list_bsdtar_dir/in/raw/images/icon.svg\n",
        "d---------   0                      0 Jan  1  2023 list_bsdtar_dir/in/raw/parent/\n",
        "----------   0                      0 Jan  1  2023 list_bsdtar_dir/in/raw/parent/child.txt\n",
        "d---------   0                      0 Jan  1  2023 list_bsdtar_dir/in/raw/pna/\n",
        "----------   0                     40 Jan  1  2023 list_bsdtar_dir/in/raw/pna/empty.pna\n",
        "----------   0                  57032 Jan  1  2023 list_bsdtar_dir/in/raw/pna/nest.pna\n",
        "----------   0                     10 Jan  1  2023 list_bsdtar_dir/in/raw/text.txt\n",
    ));
}
