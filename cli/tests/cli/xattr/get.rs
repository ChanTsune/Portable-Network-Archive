use crate::utils::{archive::for_each_entry, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn xattr_get_name_match_encoding() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_opts/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_get_opts/archive.pna",
        "--overwrite",
        "xattr_get_opts/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_get_opts/archive.pna",
        "--name",
        "user.name",
        "--value",
        "pna",
        "xattr_get_opts/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_get_opts/archive.pna",
        "--name",
        "user.value",
        "--value",
        "data",
        "xattr_get_opts/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    for_each_entry("xattr_get_opts/archive.pna", |entry| {
        match entry.header().path().as_str() {
            "xattr_get_opts/in/raw/empty.txt" => {
                assert_eq!(
                    entry.xattrs(),
                    &[pna::ExtendedAttribute::new(
                        "user.name".into(),
                        b"pna".to_vec()
                    )]
                );
            }
            "xattr_get_opts/in/raw/text.txt" => {
                assert_eq!(
                    entry.xattrs(),
                    &[pna::ExtendedAttribute::new(
                        "user.value".into(),
                        b"data".to_vec()
                    )]
                );
            }
            _ => {
                assert!(entry.xattrs().is_empty());
            }
        }
    })
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "xattr",
            "get",
            "xattr_get_opts/archive.pna",
            "xattr_get_opts/in/raw/empty.txt",
            "xattr_get_opts/in/raw/text.txt",
            "--match",
            "^user\\.",
            "--dump",
            "--encoding",
            "hex",
        ])
        .assert();

    assert.stdout(concat!(
        "# file: xattr_get_opts/in/raw/empty.txt\n",
        "user.name=0x706e61\n",
        "\n",
        "# file: xattr_get_opts/in/raw/text.txt\n",
        "user.value=0x64617461\n",
        "\n",
    ));
}
