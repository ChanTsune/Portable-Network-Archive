use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive exists but requested entry path does not.
/// Action: Run `pna xattr set` with a non-existent entry path.
/// Expectation: The command returns an error.
#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "xattr_missing_set/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_missing_set/archive.pna",
        "--overwrite",
        "xattr_missing_set/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_missing_set/archive.pna",
        "--name",
        "user.test",
        "--value",
        "test_value",
        "xattr_missing_set/in/raw/empty.txt",
        "xattr_missing_set/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}
