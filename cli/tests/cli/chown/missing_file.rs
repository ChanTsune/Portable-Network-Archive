use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "chown_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chown_missing/archive.pna",
        "--overwrite",
        "chown_missing/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_missing/archive.pna",
        "test_user:test_group",
        "chown_missing/in/raw/empty.txt",
        "chown_missing/in/raw/not_found.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}
