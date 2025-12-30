use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::collections::HashSet;
use std::fs;

/// Precondition: A pre-generated encrypted archive exists.
/// Action: Run `pna experimental delete` with `--password-file` to delete entries.
/// Expectation: Only the entries specified are removed from the encrypted archive; all
///         other entries remain.
#[test]
fn delete_with_password_file() {
    setup();
    // Use pre-generated encrypted archive (password: "password")
    TestResources::extract_in("zstd_aes_ctr.pna", "delete_password_file/").unwrap();
    let password_file_path = "delete_password_file/password_file";
    let password = "password";
    fs::write(password_file_path, password).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_password_file/zstd_aes_ctr.pna",
        "**/empty.txt",
        "--password-file",
        password_file_path,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry_with_password(
        "delete_password_file/zstd_aes_ctr.pna",
        password,
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "raw/first/second/third/pna.txt",
        "raw/images/icon.png",
        "raw/images/icon.bmp",
        "raw/images/icon.svg",
        "raw/pna/empty.pna",
        "raw/parent/child.txt",
        "raw/pna/nest.pna",
        "raw/text.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
