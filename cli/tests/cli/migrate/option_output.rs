use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

const FIXTURE: &str = "migrate_overwrite/0.33.0/zstd_keep_all.pna";

/// Precondition: A legacy archive and a pre-existing output file exist.
/// Action: Run `pna migrate` with `--output` pointing at the existing file.
/// Expectation: The command fails without touching either file.
#[test]
#[allow(deprecated)]
fn migrate_output_without_overwrite_refuses_to_clobber() {
    setup();
    TestResources::extract_in("0.33.0/zstd_keep_all.pna", "migrate_overwrite/").unwrap();
    fs::write("migrate_overwrite/out.pna", b"sentinel").unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "migrate",
        "-f",
        FIXTURE,
        "--output",
        "migrate_overwrite/out.pna",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    assert!(format!("{error:?}").contains("already exists"));
    assert_eq!(fs::read("migrate_overwrite/out.pna").unwrap(), b"sentinel");
}

/// Precondition: A legacy archive and a pre-existing output file exist.
/// Action: Run the same migrate with `--overwrite`.
/// Expectation: The output is replaced with the migrated archive.
#[test]
#[allow(deprecated)]
fn migrate_output_with_overwrite_replaces() {
    setup();
    TestResources::extract_in("0.33.0/zstd_keep_all.pna", "migrate_overwrite_ok/").unwrap();
    let fixture = "migrate_overwrite_ok/0.33.0/zstd_keep_all.pna";
    fs::write("migrate_overwrite_ok/out.pna", b"sentinel").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "migrate",
        "-f",
        fixture,
        "--output",
        "migrate_overwrite_ok/out.pna",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_ne!(
        fs::read("migrate_overwrite_ok/out.pna").unwrap(),
        b"sentinel"
    );
    let mut count = 0usize;
    archive::for_each_entry("migrate_overwrite_ok/out.pna", |_| count += 1).unwrap();
    assert!(count > 0, "migrated archive should contain entries");
}
