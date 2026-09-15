use crate::utils::{
    EmbedExt, TestResources,
    archive::{get_archive_entry_names, xattr, xattrs_by_entry},
    setup,
};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive entry has an extended attribute set.
/// Action: Remove the xattr using `--remove` option.
/// Expectation: The xattr is removed from the target entry; other entries remain unaffected.
#[test]
fn archive_xattr_remove() {
    setup();
    TestResources::extract_in("raw/", "xattr_remove/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "xattr_remove/xattr_remove.pna",
        "--overwrite",
        "xattr_remove/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "--overwrite",
        "-f",
        "xattr_remove/xattr_remove.pna",
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "xattr_remove/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        xattrs_by_entry("xattr_remove/xattr_remove.pna", None),
        vec![(
            "xattr_remove/in/raw/empty.txt".to_string(),
            vec![xattr("user.name", b"pna developers!")]
        )]
    );
    let entries = get_archive_entry_names("xattr_remove/xattr_remove.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "--overwrite",
        "-f",
        "xattr_remove/xattr_remove.pna",
        "--remove",
        "user.name",
        "xattr_remove/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        xattrs_by_entry("xattr_remove/xattr_remove.pna", None),
        vec![]
    );
    assert_eq!(
        get_archive_entry_names("xattr_remove/xattr_remove.pna"),
        entries
    );
}
