use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_chown() {
    setup();
    TestResources::extract_in("raw/", concat!(env!("CARGO_TARGET_TMPDIR"), "/chown/in/")).unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chown/chown.pna"),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chown/in/"),
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chown/chown.pna"),
        "user:group",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chown/in/raw/text.txt"),
    ]))
    .unwrap();
}
