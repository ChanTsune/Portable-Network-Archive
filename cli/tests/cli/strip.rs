use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_strip_metadata() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_strip_metadata/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_strip_metadata/strip_metadata.pna"
        ),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_strip_metadata/in/"),
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
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_strip_metadata/strip_metadata.pna"
        ),
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
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_strip_metadata/strip_metadata.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_strip_metadata/out/"),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_strip_metadata/in/"
        ))
        .to_string(),
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_strip_metadata/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_strip_metadata/out/"),
    )
    .unwrap();
}
