use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An encrypted archive exists with preserved metadata.
/// Action: Run `pna list -l` with password.
/// Expectation: Entries are listed with encryption, compression, permissions, timestamps.
#[test]
fn list_encrypted() {
    setup();
    TestResources::extract_in("zstd_aes_ctr.pna", "").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "-l",
            "-f",
            "zstd_aes_ctr.pna",
            "--password",
            "password",
        ])
        .assert();

    assert.stdout(concat!(
        "aes(ctr) zstandard .---------  -    25 - - - raw/empty.txt                  \n",
        "aes(ctr) zstandard .---------  - 35142 - - - raw/images/icon.png            \n",
        "aes(ctr) zstandard .---------  -   751 - - - raw/images/icon.svg            \n",
        "aes(ctr) zstandard .---------  - 13378 - - - raw/images/icon.bmp            \n",
        "aes(ctr) zstandard .---------  -    28 - - - raw/first/second/third/pna.txt \n",
        "aes(ctr) zstandard .---------  -    65 - - - raw/pna/empty.pna              \n",
        "aes(ctr) zstandard .---------  - 53757 - - - raw/pna/nest.pna               \n",
        "aes(ctr) zstandard .---------  -    25 - - - raw/parent/child.txt           \n",
        "aes(ctr) zstandard .---------  -    35 - - - raw/text.txt                   \n",
    ));
}

/// Precondition: A solid encrypted archive exists with preserved metadata.
/// Action: Run `pna list -l --solid` with password.
/// Expectation: Solid entries are listed with encryption details.
#[test]
fn list_encrypted_solid() {
    setup();
    TestResources::extract_in("solid_zstd_aes_ctr.pna", "").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "-l",
            "-f",
            "solid_zstd_aes_ctr.pna",
            "--solid",
            "--password",
            "password",
        ])
        .assert();

    assert.stdout(concat!(
        "aes(ctr) zstandard(solid) .---------        0       0 - - Jan  1  2023 raw/parent/child.txt           \n",
        "aes(ctr) zstandard(solid) .---------        3       3 - - Jan  1  2023 raw/first/second/third/pna.txt \n",
        "aes(ctr) zstandard(solid) .---------       40      40 - - Jan  1  2023 raw/pna/empty.pna              \n",
        "aes(ctr) zstandard(solid) .---------    51475   51475 - - Jan  1  2023 raw/images/icon.png            \n",
        "aes(ctr) zstandard(solid) .---------        0       0 - - Jan  1  2023 raw/empty.txt                  \n",
        "aes(ctr) zstandard(solid) .---------    57032   57032 - - Jan  1  2023 raw/pna/nest.pna               \n",
        "aes(ctr) zstandard(solid) .---------     1984    1984 - - Jan  1  2023 raw/images/icon.svg            \n",
        "aes(ctr) zstandard(solid) .---------       10      10 - - Jan  1  2023 raw/text.txt                   \n",
        "aes(ctr) zstandard(solid) .---------  4194442 4194442 - - Jan  1  2023 raw/images/icon.bmp            \n",
    ));
}

/// Precondition: An archive contains multiple file entries with preserved timestamps.
/// Action: Run `pna list --format table`.
/// Expectation: Entries are displayed in table format with encryption, compression, permissions, sizes, owner, group, timestamp, and filename.
#[test]
fn list_format_table() {
    setup();
    TestResources::extract_in("raw/", "list_format_table/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_table/archive.pna",
        "--overwrite",
        "--keep-timestamp",
        "--mtime",
        "2023-01-01 00:00:00",
        "list_format_table/in/",
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
        "list_format_table/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "table",
            "list_format_table/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "- zstandard .---------        0     9 - - Jan  1  2023 list_format_table/in/raw/empty.txt                  \n",
        "- zstandard .---------        3    12 - - Jan  1  2023 list_format_table/in/raw/first/second/third/pna.txt \n",
        "- zstandard .---------  4194442 17183 - - Jan  1  2023 list_format_table/in/raw/images/icon.bmp            \n",
        "- zstandard .---------    51475 38437 - - Jan  1  2023 list_format_table/in/raw/images/icon.png            \n",
        "- zstandard .---------     1984   788 - - Jan  1  2023 list_format_table/in/raw/images/icon.svg            \n",
        "- zstandard .---------        0     9 - - Jan  1  2023 list_format_table/in/raw/parent/child.txt           \n",
        "- zstandard .---------       40    49 - - Jan  1  2023 list_format_table/in/raw/pna/empty.pna              \n",
        "- zstandard .---------    57032 57041 - - Jan  1  2023 list_format_table/in/raw/pna/nest.pna               \n",
        "- zstandard .---------       10    19 - - Jan  1  2023 list_format_table/in/raw/text.txt                   \n",
    ));
}

/// Precondition: A solid archive contains multiple file entries with preserved timestamps.
/// Action: Run `pna list --format table --solid`.
/// Expectation: Solid entries are displayed in table format.
#[test]
fn list_format_table_solid() {
    setup();
    TestResources::extract_in("raw/", "list_format_table_solid/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_table_solid/archive.pna",
        "--overwrite",
        "--solid",
        "--keep-timestamp",
        "--mtime",
        "2023-01-01 00:00:00",
        "list_format_table_solid/in/",
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
        "list_format_table_solid/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "table",
            "--solid",
            "list_format_table_solid/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "- - .---------        0       0 - - Jan  1  2023 list_format_table_solid/in/raw/empty.txt                  \n",
        "- - .---------        3       3 - - Jan  1  2023 list_format_table_solid/in/raw/first/second/third/pna.txt \n",
        "- - .---------  4194442 4194442 - - Jan  1  2023 list_format_table_solid/in/raw/images/icon.bmp            \n",
        "- - .---------    51475   51475 - - Jan  1  2023 list_format_table_solid/in/raw/images/icon.png            \n",
        "- - .---------     1984    1984 - - Jan  1  2023 list_format_table_solid/in/raw/images/icon.svg            \n",
        "- - .---------        0       0 - - Jan  1  2023 list_format_table_solid/in/raw/parent/child.txt           \n",
        "- - .---------       40      40 - - Jan  1  2023 list_format_table_solid/in/raw/pna/empty.pna              \n",
        "- - .---------    57032   57032 - - Jan  1  2023 list_format_table_solid/in/raw/pna/nest.pna               \n",
        "- - .---------       10      10 - - Jan  1  2023 list_format_table_solid/in/raw/text.txt                   \n",
    ));
}

/// Precondition: An archive contains directories and files with --keep-dir.
/// Action: Run `pna list --format table`.
/// Expectation: Directories are displayed with 'd' permission prefix.
#[test]
fn list_format_table_with_directories() {
    setup();
    TestResources::extract_in("raw/", "list_table_dir/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_table_dir/archive.pna",
        "--overwrite",
        "--keep-dir",
        "--keep-timestamp",
        "--mtime",
        "2023-01-01 00:00:00",
        "list_table_dir/in/",
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
        "list_table_dir/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "table",
            "list_table_dir/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "- -         d---------        -     0 - - Jan  1  2023 list_table_dir/in                                \n",
        "- -         d---------        -     0 - - Jan  1  2023 list_table_dir/in/raw                            \n",
        "- zstandard .---------        0     9 - - Jan  1  2023 list_table_dir/in/raw/empty.txt                  \n",
        "- -         d---------        -     0 - - Jan  1  2023 list_table_dir/in/raw/first                      \n",
        "- -         d---------        -     0 - - Jan  1  2023 list_table_dir/in/raw/first/second               \n",
        "- -         d---------        -     0 - - Jan  1  2023 list_table_dir/in/raw/first/second/third         \n",
        "- zstandard .---------        3    12 - - Jan  1  2023 list_table_dir/in/raw/first/second/third/pna.txt \n",
        "- -         d---------        -     0 - - Jan  1  2023 list_table_dir/in/raw/images                     \n",
        "- zstandard .---------  4194442 17183 - - Jan  1  2023 list_table_dir/in/raw/images/icon.bmp            \n",
        "- zstandard .---------    51475 38437 - - Jan  1  2023 list_table_dir/in/raw/images/icon.png            \n",
        "- zstandard .---------     1984   788 - - Jan  1  2023 list_table_dir/in/raw/images/icon.svg            \n",
        "- -         d---------        -     0 - - Jan  1  2023 list_table_dir/in/raw/parent                     \n",
        "- zstandard .---------        0     9 - - Jan  1  2023 list_table_dir/in/raw/parent/child.txt           \n",
        "- -         d---------        -     0 - - Jan  1  2023 list_table_dir/in/raw/pna                        \n",
        "- zstandard .---------       40    49 - - Jan  1  2023 list_table_dir/in/raw/pna/empty.pna              \n",
        "- zstandard .---------    57032 57041 - - Jan  1  2023 list_table_dir/in/raw/pna/nest.pna               \n",
        "- zstandard .---------       10    19 - - Jan  1  2023 list_table_dir/in/raw/text.txt                   \n",
    ));
}
