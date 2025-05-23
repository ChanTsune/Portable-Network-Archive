use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_chown() {
    setup();
    TestResources::extract_in("raw/", "chown/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "chown/chown.pna",
        "--overwrite",
        "chown/in/",
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
        "chown/chown.pna",
        "user:group",
        "chown/in/raw/text.txt",
    ]))
    .unwrap();
}
