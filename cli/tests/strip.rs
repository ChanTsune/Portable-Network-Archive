use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_strip_metadata() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/strip_metadata.pna", env!("CARGO_TARGET_TMPDIR")),
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
        "--quiet",
        "experimental",
        "strip",
        &format!("{}/strip_metadata.pna", env!("CARGO_TARGET_TMPDIR")),
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/strip_metadata.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/strip_metadata/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]))
    .unwrap();
}
