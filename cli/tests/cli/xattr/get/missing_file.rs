use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive exists but requested entry path does not.
/// Action: Run `pna xattr get` with a non-existent entry path.
/// Expectation: The command returns an error.
#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "xattr_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_missing/archive.pna",
        "--overwrite",
        "xattr_missing/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "get",
        "xattr_missing/archive.pna",
        "xattr_missing/in/raw/empty.txt",
        "xattr_missing/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}
