use crate::utils::setup;
use portable_network_archive::cli;

#[test]
fn stdio_extract_accepts_o_flag() {
    setup();
    cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "stdio",
        "--extract",
        "--file",
        "test.pna",
        "-o",
    ])
    .unwrap();
}

#[test]
fn stdio_create_accepts_no_xattrs() {
    setup();
    cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "stdio",
        "--create",
        "--file",
        "test.pna",
        "--no-xattrs",
    ])
    .unwrap();
}

#[test]
fn stdio_xattrs_flags_conflict() {
    setup();
    let result = cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "stdio",
        "--create",
        "--file",
        "test.pna",
        "--xattrs",
        "--no-xattrs",
    ]);
    assert!(result.is_err());
}

#[test]
fn stdio_no_acls_flag_parses() {
    setup();
    cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "stdio",
        "--create",
        "--file",
        "test.pna",
        "--no-acls",
        "--unstable",
    ])
    .unwrap();
}
