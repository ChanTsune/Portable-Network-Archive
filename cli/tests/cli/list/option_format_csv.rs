use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list --format csv`.
/// Expectation: Output is valid CSV with header row and entry data.
#[test]
fn list_format_csv() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_format_csv/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "csv",
            "-f",
            "list_format_csv/zstd_with_raw_file_size.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "filename,permissions,owner,group,raw_size,compressed_size,encryption,compression,Modified\n",
        "raw/images/icon.png,---------- ,,,51475,38437,-,zstandard,\n",
        "raw/empty.txt,---------- ,,,0,9,-,zstandard,\n",
        "raw/images/icon.svg,---------- ,,,1984,789,-,zstandard,\n",
        "raw/first/second/third/pna.txt,---------- ,,,3,12,-,zstandard,\n",
        "raw/pna/empty.pna,---------- ,,,40,49,-,zstandard,\n",
        "raw/parent/child.txt,---------- ,,,0,9,-,zstandard,\n",
        "raw/pna/nest.pna,---------- ,,,57032,57041,-,zstandard,\n",
        "raw/text.txt,---------- ,,,10,19,-,zstandard,\n",
        "raw/images/icon.bmp,---------- ,,,4194442,17183,-,zstandard,\n",
    ));
}

/// Precondition: A solid archive contains multiple file entries.
/// Action: Run `pna list --format csv --solid`.
/// Expectation: Solid entries are output as valid CSV.
#[test]
fn list_format_csv_solid() {
    setup();
    TestResources::extract_in("solid_zstd.pna", "list_format_csv_solid/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "csv",
            "--solid",
            "-f",
            "list_format_csv_solid/solid_zstd.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "filename,permissions,owner,group,raw_size,compressed_size,encryption,compression,Modified\n",
        "raw/empty.txt,---------- ,,,0,0,-,zstandard(solid),\n",
        "raw/parent/child.txt,---------- ,,,0,0,-,zstandard(solid),\n",
        "raw/images/icon.svg,---------- ,,,1984,1984,-,zstandard(solid),\n",
        "raw/first/second/third/pna.txt,---------- ,,,3,3,-,zstandard(solid),\n",
        "raw/images/icon.png,---------- ,,,51475,51475,-,zstandard(solid),\n",
        "raw/pna/nest.pna,---------- ,,,57032,57032,-,zstandard(solid),\n",
        "raw/text.txt,---------- ,,,10,10,-,zstandard(solid),\n",
        "raw/pna/empty.pna,---------- ,,,40,40,-,zstandard(solid),\n",
        "raw/images/icon.bmp,---------- ,,,4194442,4194442,-,zstandard(solid),\n",
    ));
}

/// Precondition: An encrypted archive contains file entries.
/// Action: Run `pna list --format csv` with password.
/// Expectation: Entries show encryption type in CSV output.
#[test]
fn list_format_csv_encrypted() {
    setup();
    TestResources::extract_in("zstd_aes_ctr.pna", "list_format_csv_encrypted/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "csv",
            "-f",
            "list_format_csv_encrypted/zstd_aes_ctr.pna",
            "--password",
            "password",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "filename,permissions,owner,group,raw_size,compressed_size,encryption,compression,Modified\n",
        "raw/empty.txt,---------- ,,,0,25,aes(ctr),zstandard,\n",
        "raw/images/icon.png,---------- ,,,0,35142,aes(ctr),zstandard,\n",
        "raw/images/icon.svg,---------- ,,,0,751,aes(ctr),zstandard,\n",
        "raw/images/icon.bmp,---------- ,,,0,13378,aes(ctr),zstandard,\n",
        "raw/first/second/third/pna.txt,---------- ,,,0,28,aes(ctr),zstandard,\n",
        "raw/pna/empty.pna,---------- ,,,0,65,aes(ctr),zstandard,\n",
        "raw/pna/nest.pna,---------- ,,,0,53757,aes(ctr),zstandard,\n",
        "raw/parent/child.txt,---------- ,,,0,25,aes(ctr),zstandard,\n",
        "raw/text.txt,---------- ,,,0,35,aes(ctr),zstandard,\n",
    ));
}
