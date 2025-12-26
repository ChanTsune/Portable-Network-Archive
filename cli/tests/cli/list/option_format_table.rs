use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An encrypted archive exists with preserved metadata.
/// Action: Run `pna list -l` with password.
/// Expectation: Entries are listed with encryption, compression, permissions, timestamps.
#[test]
fn list_encrypted() {
    setup();
    TestResources::extract_in("zstd_aes_ctr.pna", "list_encrypted/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "-l",
            "-f",
            "list_encrypted/zstd_aes_ctr.pna",
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
    TestResources::extract_in("solid_zstd_aes_ctr.pna", "list_encrypted_solid/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "-l",
            "-f",
            "list_encrypted_solid/solid_zstd_aes_ctr.pna",
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
    TestResources::extract_in("zstd_keep_all.pna", "list_format_table/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "table",
            "-f",
            "list_format_table/zstd_keep_all.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "- -         drwxr-xr-x        -     0 root root Jan 26  2025 raw/images                     \n",
        "- -         drwxr-xr-x        -     0 root root Jan 26  2025 raw                            \n",
        "- -         drwxr-xr-x        -     0 root root Jan 26  2025 raw/first/second/third         \n",
        "- -         drwxr-xr-x        -     0 root root Jan 26  2025 raw/pna                        \n",
        "- -         drwxr-xr-x        -     0 root root Jan 26  2025 raw/first/second               \n",
        "- -         drwxr-xr-x        -     0 root root Jan 26  2025 raw/first                      \n",
        "- -         drwxr-xr-x        -     0 root root Jan 26  2025 raw/parent                     \n",
        "- zstandard .rw-r--r--        0     9 root root Jan 26  2025 raw/empty.txt                  \n",
        "- zstandard .rw-r--r--        0     9 root root Jan 26  2025 raw/parent/child.txt           \n",
        "- zstandard .rw-r--r--       40    49 root root Jan 26  2025 raw/pna/empty.pna              \n",
        "- zstandard .rw-r--r--     1984   788 root root Jan 26  2025 raw/images/icon.svg            \n",
        "- zstandard .rw-r--r--       10    19 root root Jan 26  2025 raw/text.txt                   \n",
        "- zstandard .rw-r--r--    51475 38437 root root Jan 26  2025 raw/images/icon.png            \n",
        "- zstandard .rw-r--r--    57032 57041 root root Jan 26  2025 raw/pna/nest.pna               \n",
        "- zstandard .rw-r--r--        3    12 root root Jan 26  2025 raw/first/second/third/pna.txt \n",
        "- zstandard .rw-r--r--  4194442 17183 root root Jan 26  2025 raw/images/icon.bmp            \n",
    ));
}

/// Precondition: An archive contains multiple file entries with preserved timestamps.
/// Action: Run `pna list --format table` with positional arguments to filter entries.
/// Expectation: Only matching entries are displayed in table format.
#[test]
fn list_format_table_with_filter() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_format_table_filter/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "table",
            "-f",
            "list_format_table_filter/zstd_keep_all.pna",
            "--unstable",
            "raw/text.txt",
            "raw/empty.txt",
        ])
        .assert();

    assert.stdout(concat!(
        "- zstandard .rw-r--r--   0  9 root root Jan 26  2025 raw/empty.txt \n",
        "- zstandard .rw-r--r--  10 19 root root Jan 26  2025 raw/text.txt  \n",
    ));
}

/// Precondition: An archive contains directory entries with preserved timestamps.
/// Action: Run `pna list --format table` with a directory path as positional argument.
/// Expectation: Only entries under the specified directory are displayed in table format.
#[test]
fn list_format_table_with_directory_filter() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_format_table_dir_filter/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "table",
            "-f",
            "list_format_table_dir_filter/zstd_keep_all.pna",
            "--unstable",
            "raw/images/",
        ])
        .assert();

    assert.stdout(concat!(
        "- -         drwxr-xr-x        -     0 root root Jan 26  2025 raw/images          \n",
        "- zstandard .rw-r--r--     1984   788 root root Jan 26  2025 raw/images/icon.svg \n",
        "- zstandard .rw-r--r--    51475 38437 root root Jan 26  2025 raw/images/icon.png \n",
        "- zstandard .rw-r--r--  4194442 17183 root root Jan 26  2025 raw/images/icon.bmp \n",
    ));
}

/// Precondition: A solid archive contains multiple file entries with preserved timestamps.
/// Action: Run `pna list --format table --solid`.
/// Expectation: Solid entries are displayed in table format.
#[test]
fn list_format_table_solid() {
    setup();
    TestResources::extract_in("solid_zstd_keep_all.pna", "list_format_table_solid/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "table",
            "--solid",
            "-f",
            "list_format_table_solid/solid_zstd_keep_all.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "- zstandard(solid) drwxr-xr-x        -       0 root root Jan 26  2025 raw                            \n",
        "- zstandard(solid) .rw-r--r--        0       0 root root Jan 26  2025 raw/empty.txt                  \n",
        "- zstandard(solid) drwxr-xr-x        -       0 root root Jan 26  2025 raw/first                      \n",
        "- zstandard(solid) drwxr-xr-x        -       0 root root Jan 26  2025 raw/first/second               \n",
        "- zstandard(solid) drwxr-xr-x        -       0 root root Jan 26  2025 raw/images                     \n",
        "- zstandard(solid) .rw-r--r--     1984    1984 root root Jan 26  2025 raw/images/icon.svg            \n",
        "- zstandard(solid) drwxr-xr-x        -       0 root root Jan 26  2025 raw/first/second/third         \n",
        "- zstandard(solid) drwxr-xr-x        -       0 root root Jan 26  2025 raw/pna                        \n",
        "- zstandard(solid) .rw-r--r--    51475   51475 root root Jan 26  2025 raw/images/icon.png            \n",
        "- zstandard(solid) drwxr-xr-x        -       0 root root Jan 26  2025 raw/parent                     \n",
        "- zstandard(solid) .rw-r--r--        3       3 root root Jan 26  2025 raw/first/second/third/pna.txt \n",
        "- zstandard(solid) .rw-r--r--        0       0 root root Jan 26  2025 raw/parent/child.txt           \n",
        "- zstandard(solid) .rw-r--r--       40      40 root root Jan 26  2025 raw/pna/empty.pna              \n",
        "- zstandard(solid) .rw-r--r--       10      10 root root Jan 26  2025 raw/text.txt                   \n",
        "- zstandard(solid) .rw-r--r--    57032   57032 root root Jan 26  2025 raw/pna/nest.pna               \n",
        "- zstandard(solid) .rw-r--r--  4194442 4194442 root root Jan 26  2025 raw/images/icon.bmp            \n",
    ));
}
