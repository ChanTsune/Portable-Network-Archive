use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn delete_overwrite() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/delete_overwrite.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        &format!("{}/delete_overwrite.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
    ]))
    .unwrap();
}

#[test]
fn delete_output() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/delete_output.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        &format!("{}/delete_output.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
        "--output",
        &format!("{}/delete_output/deleted.pna", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}

#[test]
fn delete_output_exclude() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/delete_output_exclude.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        &format!("{}/delete_output_exclude.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
        "--exclude",
        "resource/test/raw/**",
        "--unstable",
        "--output",
        &format!(
            "{}/delete_output_exclude/delete_excluded.pna",
            env!("CARGO_TARGET_TMPDIR")
        ),
    ]))
    .unwrap();
}

#[test]
fn delete_solid() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/delete_solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--solid",
        "-r",
        "../resources/test/raw",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        &format!("{}/delete_solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/delete_solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/delete_solid/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}

#[test]
fn delete_unsolid() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/delete_unsolid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--solid",
        "-r",
        "../resources/test/raw",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "--unsolid",
        &format!("{}/delete_unsolid.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/text.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/delete_unsolid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/delete_unsolid/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}
