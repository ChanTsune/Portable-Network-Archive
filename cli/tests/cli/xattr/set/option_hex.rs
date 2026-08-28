use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive with multiple entries exists.
/// Action: Set an xattr with hex-encoded value (0x prefix).
/// Expectation: Target entry has the decoded value; other entries remain unaffected.
#[test]
fn xattr_set_hex() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_set_hex/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
        "xattr_set_hex/zstd.pna",
        "--name",
        "user.hex",
        "--value",
        "0x48656c6c6f20576f726c64",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::xattrs_by_entry("xattr_set_hex/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.hex", b"Hello World")]
        )]
    );
}
