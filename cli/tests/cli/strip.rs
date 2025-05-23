use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_strip_metadata() {
    setup();
    TestResources::extract_in("raw/", "archive_strip_metadata/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_strip_metadata/strip_metadata.pna",
        "--overwrite",
        "archive_strip_metadata/in/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "strip",
        "archive_strip_metadata/strip_metadata.pna",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_strip_metadata/strip_metadata.pna",
        "--overwrite",
        "--out-dir",
        "archive_strip_metadata/out/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--strip-components",
        &components_count("archive_strip_metadata/in/").to_string(),
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();

    diff("archive_strip_metadata/in/", "archive_strip_metadata/out/").unwrap();
}
