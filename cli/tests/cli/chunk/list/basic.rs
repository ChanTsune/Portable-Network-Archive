//! Tests for basic chunk list functionality.

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An empty PNA archive exists (only AHED and AEND chunks).
/// Action: Run `pna experimental chunk list -f <archive>`.
/// Expectation: Output shows exactly the archive header and end chunks.
#[test]
fn chunk_list_empty_archive() {
    setup();
    TestResources::extract_in("empty.pna", "chunk_list_empty/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-f",
            "chunk_list_empty/empty.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(" 1  AHED  8  0x0008 \n", " 2  AEND  0  0x001c \n",));
}

/// Precondition: A deflate-compressed PNA archive exists.
/// Action: Run `pna experimental chunk list -f <archive>`.
/// Expectation: Output shows AHED, FHED, FDAT, FEND chunks in correct order.
#[test]
fn chunk_list_deflate_archive() {
    setup();
    TestResources::extract_in("deflate.pna", "chunk_list_deflate/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-f",
            "chunk_list_deflate/deflate.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            " 1   AHED  8      0x0008  \n",
            " 2   FHED  19     0x001c  \n",
            " 3   FDAT  8      0x003b  \n",
            " 4   FEND  0      0x004f  \n",
            " 5   FHED  25     0x005b  \n",
            " 6   FDAT  35894  0x0080  \n",
            " 7   FEND  0      0x8cc2  \n",
            " 8   FHED  25     0x8cce  \n",
            " 9   FDAT  781    0x8cf3  \n",
            " 10  FEND  0      0x900c  \n",
            " 11  FHED  25     0x9018  \n",
            " 12  FDAT  28085  0x903d  \n",
            " 13  FEND  0      0xfdfe  \n",
            " 14  FHED  36     0xfe0a  \n",
            " 15  FDAT  11     0xfe3a  \n",
            " 16  FEND  0      0xfe51  \n",
            " 17  FHED  23     0xfe5d  \n",
            " 18  FDAT  40     0xfe80  \n",
            " 19  FEND  0      0xfeb4  \n",
            " 20  FHED  22     0xfec0  \n",
            " 21  FDAT  54167  0xfee2  \n",
            " 22  FEND  0      0x1d285 \n",
            " 23  FHED  26     0x1d291 \n",
            " 24  FDAT  8      0x1d2b7 \n",
            " 25  FEND  0      0x1d2cb \n",
            " 26  FHED  18     0x1d2d7 \n",
            " 27  FDAT  18     0x1d2f5 \n",
            " 28  FEND  0      0x1d313 \n",
            " 29  AEND  0      0x1d31f \n",
        ));
}
