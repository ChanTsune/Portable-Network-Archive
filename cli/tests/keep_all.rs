use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_keep_all() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/keep_all.pna", env!("CARGO_TARGET_TMPDIR")),
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
        "x",
        &format!("{}/keep_all.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/keep_all/", env!("CARGO_TARGET_TMPDIR")),
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
