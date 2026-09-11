use crate::utils::{archive, setup};
use clap::Parser;
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
/// Action: Run concat with `--output` pointing at one of the inputs.
/// Expectation: Command succeeds and the output contains entries from
/// every input instead of truncating the overlapped input before reading.
#[test]
fn explicit_output_same_as_input() {
    setup();
    let _ = fs::remove_dir_all("concat_same_output");
    create_fixture("concat_same_output", "a", b"a-content");
    create_fixture("concat_same_output", "b", b"b-content");

    portable_network_archive::cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "concat",
        "--files",
        "concat_same_output/a.pna",
        "--files",
        "concat_same_output/b.pna",
        "--output",
        "concat_same_output/a.pna",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("concat_same_output/a.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert!(
        seen.iter().any(|p| p.ends_with('a')),
        "output must contain first input: {seen:?}"
    );
    assert!(
        seen.iter().any(|p| p.ends_with('b')),
        "output must contain second input: {seen:?}"
    );
}
