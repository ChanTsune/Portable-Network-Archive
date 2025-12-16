use crate::utils::{self, EmbedExt, TestResources, diff::diff, setup};
use clap::Parser;
use portable_network_archive::cli;

#[test]
fn create_with_include() {
    setup();
    TestResources::extract_in("raw/", "create_with_include/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_include/include.pna",
        "--overwrite",
        "create_with_include/in/",
        "--include",
        "**/*.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_include/include.pna",
        "--overwrite",
        "--out-dir",
        "create_with_include/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let excluded = [
        "create_with_include/in/raw/images/icon.bmp",
        "create_with_include/in/raw/images/icon.png",
        "create_with_include/in/raw/images/icon.svg",
        "create_with_include/in/raw/pna/empty.pna",
        "create_with_include/in/raw/pna/nest.pna",
    ];
    for file in excluded {
        utils::remove_with_empty_parents(file).unwrap();
    }

    diff("create_with_include/in/", "create_with_include/out/").unwrap();
}
