use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_chmod() {
    setup();
    TestResources::extract_in("raw/", "chmod/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod/chmod.pna",
        "--overwrite",
        "chmod/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "chmod/chmod.pna",
        "--",
        "-w",
        "chmod/in/raw/text.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "chmod/chmod.pna",
        "+w",
        "chmod/in/raw/text.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "chmod/chmod.pna",
        "--overwrite",
        "--out-dir",
        "chmod/out/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
        "--strip-components",
        "2",
    ]))
    .unwrap();

    diff("chmod/in/", "chmod/out/").unwrap();
}
