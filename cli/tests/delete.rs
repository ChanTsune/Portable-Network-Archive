use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn delete_overwrite() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/delete_overwrite.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        &format!("{}/delete_overwrite.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn delete_output() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/delete_output.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        &format!("{}/delete_output.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
        "--password",
        "password",
        "--output",
        &format!("{}/delete_output/deleted.pna", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}
