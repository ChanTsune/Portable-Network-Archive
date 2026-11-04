//! Tests for chunk listing of solid archives.

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: A solid deflate-compressed archive exists.
/// Action: Run `pna experimental chunk list -f <solid_archive>`.
/// Expectation: Output shows solid-specific chunk types (AHED, SHED, SDAT, SEND, AEND).
#[test]
fn chunk_list_solid_deflate() {
    setup();
    TestResources::extract_in("solid_deflate.pna", "chunk_list_solid_deflate/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-f",
            "chunk_list_solid_deflate/solid_deflate.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            " 1  AHED  8      0x0008  \n",
            " 2  SHED  5      0x001c  \n",
            " 3  SDAT  32768  0x002d  \n",
            " 4  SDAT  24733  0x8039  \n",
            " 5  SDAT  32768  0xe0e2  \n",
            " 6  SDAT  12158  0x160ee \n",
            " 7  SDAT  19841  0x19078 \n",
            " 8  SEND  0      0x1de05 \n",
            " 9  AEND  0      0x1de11 \n",
        ));
}

/// Precondition: A solid deflate-compressed archive exists.
/// Action: Run `pna experimental chunk list --header -f <solid_archive>`.
/// Expectation: Output shows header row and solid chunk types.
#[test]
fn chunk_list_solid_with_header() {
    setup();
    TestResources::extract_in("solid_deflate.pna", "chunk_list_solid_header/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--header",
            "-f",
            "chunk_list_solid_header/solid_deflate.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            " Index  Type  Size   Offset  \n",
            " 1      AHED  8      0x0008  \n",
            " 2      SHED  5      0x001c  \n",
            " 3      SDAT  32768  0x002d  \n",
            " 4      SDAT  24733  0x8039  \n",
            " 5      SDAT  32768  0xe0e2  \n",
            " 6      SDAT  12158  0x160ee \n",
            " 7      SDAT  19841  0x19078 \n",
            " 8      SEND  0      0x1de05 \n",
            " 9      AEND  0      0x1de11 \n",
        ));
}
