use crate::utils::{
    diff::{diff, DiffError},
    setup, EmbedExt, TestResources,
};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn append_exclude() {
    setup();
    TestResources::extract_in("raw/", "append_exclude/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "append_exclude/append.pna",
        "--overwrite",
        "append_exclude/in/",
        "--exclude",
        "*/extra/*",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input
    TestResources::extract_in("store.pna", "append_exclude/in/extra/").unwrap();
    TestResources::extract_in("zstd.pna", "append_exclude/in/extra/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "append_exclude/append.pna",
        "append_exclude/in/extra/",
        "--exclude",
        "*/z*.pna",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "append_exclude/append.pna",
        "--overwrite",
        "--out-dir",
        "append_exclude/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    // check completely extracted
    let result = diff("append_exclude/in/", "append_exclude/out/").unwrap();

    assert_eq!(
        result,
        maplit::hashset! {
            DiffError::only_in("append_exclude/in/","extra/zstd.pna")
        }
    );
}
