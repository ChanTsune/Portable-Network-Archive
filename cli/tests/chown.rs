use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_chown() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/chown.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
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
        "experimental",
        "chown",
        &format!("{}/chown.pna", env!("CARGO_TARGET_TMPDIR")),
        "user:group",
        "resources/test/raw/text.txt",
    ]))
    .unwrap();
}
