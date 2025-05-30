use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn experimental_diff() {
    setup();
    TestResources::extract_in("raw/", "diff/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "diff/diff.pna",
        "--overwrite",
        "diff/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from(["pna", "experimental", "diff", "diff/diff.pna"])
        .unwrap()
        .execute()
        .unwrap();
}
