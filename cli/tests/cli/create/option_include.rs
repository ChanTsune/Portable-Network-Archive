use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: The source tree contains files matching and not matching the include pattern.
/// Action: Run `pna create` with `--include` on a filesystem source.
/// Expectation: The archive is empty because `--include` does not filter filesystem sources.
#[test]
fn create_with_include() {
    setup();
    TestResources::extract_in("raw/", "create_with_include/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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

    let mut count = 0usize;
    archive::for_each_entry("create_with_include/include.pna", |_| {
        count += 1;
    })
    .unwrap();
    assert_eq!(
        count, 0,
        "archive should be empty with --include on filesystem create"
    );
}
