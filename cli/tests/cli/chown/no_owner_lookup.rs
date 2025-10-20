use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashMap;

#[test]
fn chown_no_owner_lookup() {
    setup();
    TestResources::extract_in("raw/", "chown_no_owner_lookup/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chown_no_owner_lookup/no_owner_lookup.pna",
        "--overwrite",
        "chown_no_owner_lookup/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut original_owners = HashMap::new();

    archive::for_each_entry("chown_no_owner_lookup/no_owner_lookup.pna", |entry| {
        original_owners.insert(
            entry.header().path().to_string(),
            entry.metadata().permission().cloned(),
        );
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_no_owner_lookup/no_owner_lookup.pna",
        "test_user:test_group",
        "chown_no_owner_lookup/in/raw/text.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    #[cfg(not(target_family = "wasm"))]
    archive::for_each_entry(
        "chown_no_owner_lookup/no_owner_lookup.pna",
        |entry| match entry.header().path().as_str() {
            path @ "chown_no_owner_lookup/in/raw/text.txt" => {
                let permission = entry.metadata().permission().unwrap();
                let original = original_owners.get(path).unwrap().clone().unwrap();
                assert_eq!(permission.gname(), "test_group");
                assert_eq!(permission.uname(), "test_user");
                assert_eq!(permission.uid(), u64::MAX);
                assert_eq!(permission.gid(), u64::MAX);
                assert_eq!(permission.permissions(), original.permissions());
            }
            path => {
                let permission = entry.metadata().permission();
                let original = original_owners.get(path).unwrap();
                assert_eq!(permission, original.as_ref());
            }
        },
    )
    .unwrap();
}
