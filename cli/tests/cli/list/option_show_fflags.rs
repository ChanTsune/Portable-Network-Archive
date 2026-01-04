use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive with file flags exists.
/// Action: Run `pna list -l -O` to show file flags.
/// Expectation: The fflags column is displayed with flag values.
#[test]
fn list_show_fflags_table() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", "list_show_fflags_table/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "list",
            "-l",
            "-O",
            "-f",
            "list_show_fflags_table/zstd_keep_fflags.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            "- zstandard .---------  uchg               10 19 - - - file1.txt    \n",
            "- zstandard .---------  nodump             10 19 - - - file2.txt    \n",
            "- zstandard .---------  hidden,schg        10 19 - - - file3.txt    \n",
            "- zstandard .---------  hidden,nodump,uchg 13 22 - - - testfile.txt \n",
        ));
}

/// Precondition: An archive with file flags exists.
/// Action: Run `pna list -l -O --header` to show file flags with header.
/// Expectation: The header includes "Fflags" column.
#[test]
fn list_show_fflags_with_header() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", "list_show_fflags_with_header/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "list",
            "-l",
            "-O",
            "--header",
            "-f",
            "list_show_fflags_with_header/zstd_keep_fflags.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            "Encryption Compression Permissions Fflags             Raw Size Compressed Size User Group Modified Name         \n",
            "-          zstandard   .---------  uchg                     10              19 -    -     -        file1.txt    \n",
            "-          zstandard   .---------  nodump                   10              19 -    -     -        file2.txt    \n",
            "-          zstandard   .---------  hidden,schg              10              19 -    -     -        file3.txt    \n",
            "-          zstandard   .---------  hidden,nodump,uchg       13              22 -    -     -        testfile.txt \n",
        ));
}

/// Precondition: An archive with file flags exists.
/// Action: Run `pna list -O --format csv --unstable` to show file flags.
/// Expectation: The fflags are shown as a CSV column.
#[test]
fn list_show_fflags_csv() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", "list_show_fflags_csv/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "list",
            "-O",
            "--format",
            "csv",
            "--unstable",
            "-f",
            "list_show_fflags_csv/zstd_keep_fflags.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            "filename,permissions,owner,group,raw_size,compressed_size,encryption,compression,fflags,Modified\n",
            "file1.txt,---------- ,,,10,19,-,zstandard,uchg,\n",
            "file2.txt,---------- ,,,10,19,-,zstandard,nodump,\n",
            "file3.txt,---------- ,,,10,19,-,zstandard,\"hidden,schg\",\n",
            "testfile.txt,---------- ,,,13,22,-,zstandard,\"hidden,nodump,uchg\",\n",
        ));
}

/// Precondition: An archive with file flags exists.
/// Action: Run `pna list -O --format jsonl --unstable` to check JSON output.
/// Expectation: The fflags array is included in JSON output.
#[test]
fn list_fflags_jsonl() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", "list_fflags_jsonl/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "list",
            "-O",
            "--format",
            "jsonl",
            "--unstable",
            "-f",
            "list_fflags_jsonl/zstd_keep_fflags.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            r#"{"filename":"file1.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":19,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","fflags":["uchg"],"acl":[],"xattr":[]}"#,
            "\n",
            r#"{"filename":"file2.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":19,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","fflags":["nodump"],"acl":[],"xattr":[]}"#,
            "\n",
            r#"{"filename":"file3.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":19,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","fflags":["hidden","schg"],"acl":[],"xattr":[]}"#,
            "\n",
            r#"{"filename":"testfile.txt","permissions":"---------- ","owner":"","group":"","raw_size":13,"size":22,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","fflags":["hidden","nodump","uchg"],"acl":[],"xattr":[]}"#,
            "\n",
        ));
}

/// Precondition: An archive with file flags exists.
/// Action: Run `pna list --format jsonl --unstable` without -O flag.
/// Expectation: The fflags field is NOT included in JSON output.
#[test]
fn list_jsonl_without_fflags() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", "list_jsonl_without_fflags/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "list",
            "--format",
            "jsonl",
            "--unstable",
            "-f",
            "list_jsonl_without_fflags/zstd_keep_fflags.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            r#"{"filename":"file1.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":19,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
            "\n",
            r#"{"filename":"file2.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":19,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
            "\n",
            r#"{"filename":"file3.txt","permissions":"---------- ","owner":"","group":"","raw_size":10,"size":19,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
            "\n",
            r#"{"filename":"testfile.txt","permissions":"---------- ","owner":"","group":"","raw_size":13,"size":22,"encryption":"-","compression":"zstandard","created":"","modified":"","accessed":"","acl":[],"xattr":[]}"#,
            "\n",
        ));
}

/// Precondition: An archive with file flags exists.
/// Action: Run `pna list -l` without -O flag.
/// Expectation: The fflags column is NOT displayed.
#[test]
fn list_without_show_fflags() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", "list_without_show_fflags/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "list",
            "-l",
            "-f",
            "list_without_show_fflags/zstd_keep_fflags.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            "- zstandard .---------  10 19 - - - file1.txt    \n",
            "- zstandard .---------  10 19 - - - file2.txt    \n",
            "- zstandard .---------  10 19 - - - file3.txt    \n",
            "- zstandard .---------  13 22 - - - testfile.txt \n",
        ));
}
