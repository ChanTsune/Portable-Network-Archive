use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_list() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list.pna", env!("CARGO_TARGET_TMPDIR")),
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
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        &format!("{}/list.pna", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn archive_list_solid() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list_solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        &format!("{}/list_solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--solid",
        "--password",
        "password",
    ]))
    .unwrap();
}
