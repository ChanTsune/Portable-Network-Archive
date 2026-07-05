use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, path::Path};

fn create_archive(dir: &str) {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{dir}/file.txt"), "content").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &format!("{dir}/archive.pna"),
        "--overwrite",
        &format!("{dir}/file.txt"),
        "--no-keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();
}

/// Precondition: Archive entry has no mtime (mTIM chunk omitted).
/// Action: Run `pna extract` with a newer-mtime filter and `--missing-time exclude`.
/// Expectation: The mtime-missing entry is not extracted.
#[test]
fn extract_missing_time_exclude_skips_entry() {
    setup();
    create_archive("extract_missing_time_exclude");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_missing_time_exclude/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_missing_time_exclude/out/",
        "--unstable",
        "--newer-mtime",
        "@1000000000",
        "--missing-time",
        "exclude",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(
        !Path::new("extract_missing_time_exclude/out/extract_missing_time_exclude/file.txt")
            .exists(),
        "exclude policy should skip mtime-missing entries"
    );
}

/// Precondition: Archive entry has no mtime.
/// Action: Run `pna extract` with a newer-mtime filter and no `--missing-time`.
/// Expectation: Default `include` policy extracts the mtime-missing entry.
#[test]
fn extract_missing_time_default_extracts_entry() {
    setup();
    create_archive("extract_missing_time_default");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_missing_time_default/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_missing_time_default/out/",
        "--unstable",
        "--newer-mtime",
        "@1000000000",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(
        Path::new("extract_missing_time_default/out/extract_missing_time_default/file.txt")
            .exists(),
        "default include policy should extract mtime-missing entries"
    );
}
