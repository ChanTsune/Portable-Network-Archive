use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use predicates::{boolean::PredicateBooleanExt, str::contains};
use std::{collections::HashSet, fs};

fn create_fixture(dir: &str, name: &str, content: &[u8]) {
    fs::create_dir_all(format!("{dir}/in")).unwrap();
    fs::write(format!("{dir}/in/{name}"), content).unwrap();
    portable_network_archive::cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "-f",
        &format!("{dir}/{name}.pna"),
        "--overwrite",
        &format!("{dir}/in/{name}"),
    ])
    .unwrap()
    .execute()
    .unwrap();
}

/// Precondition: Two input archives exist.
/// Action: Run concat with explicit `--output`.
/// Expectation: Every input (including the first) is concatenated as an
/// input, and no deprecation warning is emitted.
#[test]
fn explicit_output_treats_every_file_as_input() {
    setup();
    let _ = fs::remove_dir_all("concat_output");
    create_fixture("concat_output", "a", b"a-content");
    create_fixture("concat_output", "b", b"b-content");

    cargo_bin_cmd!("pna")
        .args([
            "concat",
            "-f",
            "concat_output/a.pna",
            "-f",
            "concat_output/b.pna",
            "--output",
            "concat_output/out.pna",
        ])
        .assert()
        .success()
        .stderr(contains("deprecated").not());

    let mut seen = HashSet::new();
    archive::for_each_entry("concat_output/out.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert!(
        seen.iter().any(|p| p.ends_with("a")),
        "first --file must be read as an input: {seen:?}"
    );
    assert!(
        seen.iter().any(|p| p.ends_with("b")),
        "second --file must be read as an input: {seen:?}"
    );
}

/// Precondition: One input archive exists.
/// Action: Run concat without `--output` (legacy form).
/// Expectation: The first input is still used as the destination, with a
/// deprecation warning guiding toward `--output`.
#[test]
fn legacy_first_file_destination_warns_but_works() {
    setup();
    let _ = fs::remove_dir_all("concat_legacy");
    create_fixture("concat_legacy", "a", b"a-content");
    let before = fs::read("concat_legacy/a.pna").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "concat",
            "-f",
            "concat_legacy/out.pna",
            "-f",
            "concat_legacy/a.pna",
        ])
        .assert()
        .success()
        .stderr(contains("--output"));

    let mut seen = HashSet::new();
    archive::for_each_entry("concat_legacy/out.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert!(
        seen.iter().any(|p| p.ends_with("a")),
        "legacy destination must contain the input: {seen:?}"
    );
    assert_eq!(
        fs::read("concat_legacy/a.pna").unwrap(),
        before,
        "input archive must be untouched"
    );
}
