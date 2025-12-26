use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list --format tsv`.
/// Expectation: Output is valid TSV with header row and entry data.
#[test]
fn list_format_tsv() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_format_tsv/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tsv",
            "-f",
            "list_format_tsv/zstd_with_raw_file_size.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "filename\tpermissions\towner\tgroup\traw_size\tcompressed_size\tencryption\tcompression\tModified\n",
        "raw/images/icon.png\t---------- \t\t\t51475\t38437\t-\tzstandard\t\n",
        "raw/empty.txt\t---------- \t\t\t0\t9\t-\tzstandard\t\n",
        "raw/images/icon.svg\t---------- \t\t\t1984\t789\t-\tzstandard\t\n",
        "raw/first/second/third/pna.txt\t---------- \t\t\t3\t12\t-\tzstandard\t\n",
        "raw/pna/empty.pna\t---------- \t\t\t40\t49\t-\tzstandard\t\n",
        "raw/parent/child.txt\t---------- \t\t\t0\t9\t-\tzstandard\t\n",
        "raw/pna/nest.pna\t---------- \t\t\t57032\t57041\t-\tzstandard\t\n",
        "raw/text.txt\t---------- \t\t\t10\t19\t-\tzstandard\t\n",
        "raw/images/icon.bmp\t---------- \t\t\t4194442\t17183\t-\tzstandard\t\n",
    ));
}

/// Precondition: A solid archive contains multiple file entries.
/// Action: Run `pna list --format tsv --solid`.
/// Expectation: Solid entries are output as valid TSV.
#[test]
fn list_format_tsv_solid() {
    setup();
    TestResources::extract_in("solid_zstd.pna", "list_format_tsv_solid/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tsv",
            "--solid",
            "-f",
            "list_format_tsv_solid/solid_zstd.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "filename\tpermissions\towner\tgroup\traw_size\tcompressed_size\tencryption\tcompression\tModified\n",
        "raw/empty.txt\t---------- \t\t\t0\t0\t-\tzstandard(solid)\t\n",
        "raw/parent/child.txt\t---------- \t\t\t0\t0\t-\tzstandard(solid)\t\n",
        "raw/images/icon.svg\t---------- \t\t\t1984\t1984\t-\tzstandard(solid)\t\n",
        "raw/first/second/third/pna.txt\t---------- \t\t\t3\t3\t-\tzstandard(solid)\t\n",
        "raw/images/icon.png\t---------- \t\t\t51475\t51475\t-\tzstandard(solid)\t\n",
        "raw/pna/nest.pna\t---------- \t\t\t57032\t57032\t-\tzstandard(solid)\t\n",
        "raw/text.txt\t---------- \t\t\t10\t10\t-\tzstandard(solid)\t\n",
        "raw/pna/empty.pna\t---------- \t\t\t40\t40\t-\tzstandard(solid)\t\n",
        "raw/images/icon.bmp\t---------- \t\t\t4194442\t4194442\t-\tzstandard(solid)\t\n",
    ));
}

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list --format tsv` with positional arguments to filter entries.
/// Expectation: Only matching entries are output in TSV format.
#[test]
fn list_format_tsv_with_filter() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_format_tsv_filter/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tsv",
            "-f",
            "list_format_tsv_filter/zstd_with_raw_file_size.pna",
            "--unstable",
            "raw/text.txt",
            "raw/empty.txt",
        ])
        .assert();

    assert.stdout(concat!(
        "filename\tpermissions\towner\tgroup\traw_size\tcompressed_size\tencryption\tcompression\tModified\n",
        "raw/empty.txt\t---------- \t\t\t0\t9\t-\tzstandard\t\n",
        "raw/text.txt\t---------- \t\t\t10\t19\t-\tzstandard\t\n",
    ));
}

/// Precondition: An archive contains directory entries.
/// Action: Run `pna list --format tsv` with a directory path as positional argument.
/// Expectation: Only entries under the specified directory are output in TSV format.
#[test]
fn list_format_tsv_with_directory_filter() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_format_tsv_dir_filter/")
        .unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tsv",
            "-f",
            "list_format_tsv_dir_filter/zstd_with_raw_file_size.pna",
            "--unstable",
            "raw/images/",
        ])
        .assert();

    assert.stdout(concat!(
        "filename\tpermissions\towner\tgroup\traw_size\tcompressed_size\tencryption\tcompression\tModified\n",
        "raw/images/icon.png\t---------- \t\t\t51475\t38437\t-\tzstandard\t\n",
        "raw/images/icon.svg\t---------- \t\t\t1984\t789\t-\tzstandard\t\n",
        "raw/images/icon.bmp\t---------- \t\t\t4194442\t17183\t-\tzstandard\t\n",
    ));
}

/// Precondition: An encrypted archive contains file entries.
/// Action: Run `pna list --format tsv` with password.
/// Expectation: Entries show encryption type in TSV output.
#[test]
fn list_format_tsv_encrypted() {
    setup();
    TestResources::extract_in("zstd_aes_ctr.pna", "list_format_tsv_encrypted/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tsv",
            "-f",
            "list_format_tsv_encrypted/zstd_aes_ctr.pna",
            "--password",
            "password",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        "filename\tpermissions\towner\tgroup\traw_size\tcompressed_size\tencryption\tcompression\tModified\n",
        "raw/empty.txt\t---------- \t\t\t0\t25\taes(ctr)\tzstandard\t\n",
        "raw/images/icon.png\t---------- \t\t\t0\t35142\taes(ctr)\tzstandard\t\n",
        "raw/images/icon.svg\t---------- \t\t\t0\t751\taes(ctr)\tzstandard\t\n",
        "raw/images/icon.bmp\t---------- \t\t\t0\t13378\taes(ctr)\tzstandard\t\n",
        "raw/first/second/third/pna.txt\t---------- \t\t\t0\t28\taes(ctr)\tzstandard\t\n",
        "raw/pna/empty.pna\t---------- \t\t\t0\t65\taes(ctr)\tzstandard\t\n",
        "raw/pna/nest.pna\t---------- \t\t\t0\t53757\taes(ctr)\tzstandard\t\n",
        "raw/parent/child.txt\t---------- \t\t\t0\t25\taes(ctr)\tzstandard\t\n",
        "raw/text.txt\t---------- \t\t\t0\t35\taes(ctr)\tzstandard\t\n",
    ));
}
