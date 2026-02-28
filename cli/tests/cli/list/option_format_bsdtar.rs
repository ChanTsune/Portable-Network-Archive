use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains multiple file entries with preserved timestamps.
/// Action: Run `pna list --format bsdtar`.
/// Expectation: Entries are displayed in bsdtar format with permissions, size, timestamp, and filename.
#[test]
fn list_format_bsdtar() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_format_bsdtar/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "bsdtar",
            "-f",
            "list_format_bsdtar/zstd_keep_all.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/images/\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/first/second/third/\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/pna/\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/first/second/\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/first/\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/parent/\n",
        "-rw-r--r--  0 root   root        0 Jan 26  2025 raw/empty.txt\n",
        "-rw-r--r--  0 root   root        0 Jan 26  2025 raw/parent/child.txt\n",
        "-rw-r--r--  0 root   root       40 Jan 26  2025 raw/pna/empty.pna\n",
        "-rw-r--r--  0 root   root     1984 Jan 26  2025 raw/images/icon.svg\n",
        "-rw-r--r--  0 root   root       10 Jan 26  2025 raw/text.txt\n",
        "-rw-r--r--  0 root   root    51475 Jan 26  2025 raw/images/icon.png\n",
        "-rw-r--r--  0 root   root    57032 Jan 26  2025 raw/pna/nest.pna\n",
        "-rw-r--r--  0 root   root        3 Jan 26  2025 raw/first/second/third/pna.txt\n",
        "-rw-r--r--  0 root   root  4194442 Jan 26  2025 raw/images/icon.bmp\n",
    ));
}

/// Precondition: An archive contains multiple file entries with preserved timestamps.
/// Action: Run `pna list --format bsdtar` with positional arguments to filter entries.
/// Expectation: Only matching entries are displayed in bsdtar format.
#[test]
fn list_format_bsdtar_with_filter() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_format_bsdtar_filter/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "bsdtar",
            "-f",
            "list_format_bsdtar_filter/zstd_keep_all.pna",
            "--unstable",
            "raw/text.txt",
            "raw/empty.txt",
        ])
        .assert();

    assert.stdout(concat!(
        "-rw-r--r--  0 root   root        0 Jan 26  2025 raw/empty.txt\n",
        "-rw-r--r--  0 root   root       10 Jan 26  2025 raw/text.txt\n",
    ));
}

/// Precondition: An archive contains directory entries with preserved timestamps.
/// Action: Run `pna list --format bsdtar` with a directory path as positional argument.
/// Expectation: Only entries under the specified directory are displayed in bsdtar format.
#[test]
fn list_format_bsdtar_with_directory_filter() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_format_bsdtar_dir_filter/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "bsdtar",
            "-f",
            "list_format_bsdtar_dir_filter/zstd_keep_all.pna",
            "--unstable",
            "raw/images/",
        ])
        .assert();

    assert.stdout(concat!(
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/images/\n",
        "-rw-r--r--  0 root   root     1984 Jan 26  2025 raw/images/icon.svg\n",
        "-rw-r--r--  0 root   root    51475 Jan 26  2025 raw/images/icon.png\n",
        "-rw-r--r--  0 root   root  4194442 Jan 26  2025 raw/images/icon.bmp\n",
    ));
}

/// Precondition: A solid archive contains multiple file entries with preserved timestamps.
/// Action: Run `pna list --format bsdtar --solid`.
/// Expectation: Solid entries are displayed in bsdtar format.
#[test]
fn list_format_bsdtar_solid() {
    setup();
    TestResources::extract_in("solid_zstd_keep_all.pna", "list_format_bsdtar_solid/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "bsdtar",
            "--solid",
            "-f",
            "list_format_bsdtar_solid/solid_zstd_keep_all.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/\n",
        "-rw-r--r--  0 root   root        0 Jan 26  2025 raw/empty.txt\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/first/\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/first/second/\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/images/\n",
        "-rw-r--r--  0 root   root     1984 Jan 26  2025 raw/images/icon.svg\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/first/second/third/\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/pna/\n",
        "-rw-r--r--  0 root   root    51475 Jan 26  2025 raw/images/icon.png\n",
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 raw/parent/\n",
        "-rw-r--r--  0 root   root        3 Jan 26  2025 raw/first/second/third/pna.txt\n",
        "-rw-r--r--  0 root   root        0 Jan 26  2025 raw/parent/child.txt\n",
        "-rw-r--r--  0 root   root       40 Jan 26  2025 raw/pna/empty.pna\n",
        "-rw-r--r--  0 root   root       10 Jan 26  2025 raw/text.txt\n",
        "-rw-r--r--  0 root   root    57032 Jan 26  2025 raw/pna/nest.pna\n",
        "-rw-r--r--  0 root   root  4194442 Jan 26  2025 raw/images/icon.bmp\n",
    ));
}
