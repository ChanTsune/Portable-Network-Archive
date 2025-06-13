use crate::utils::{archive, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashMap;

#[test]
fn chown_no_owner_lookup_numeric() {
    setup();
    TestResources::extract_in("raw/", "chown_no_owner_lookup_numeric/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chown_no_owner_lookup_numeric/numeric_owner.pna",
        "--overwrite",
        "chown_no_owner_lookup_numeric/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut original_owners = HashMap::new();
    archive::for_each_entry("chown_no_owner_lookup_numeric/numeric_owner.pna", |entry| {
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
        "chown_no_owner_lookup_numeric/numeric_owner.pna",
        "1000:2000",
        "chown_no_owner_lookup_numeric/in/raw/text.txt",
        "--numeric-owner",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    #[cfg(not(target_family = "wasm"))]
    archive::for_each_entry(
        "chown_no_owner_lookup_numeric/numeric_owner.pna",
        |entry| match entry.header().path().as_str() {
            path @ "chown_no_owner_lookup_numeric/in/raw/text.txt" => {
                let permission = entry.metadata().permission().unwrap();
                let original = original_owners.get(path).unwrap().clone().unwrap();
                assert_eq!(permission.gname(), "");
                assert_eq!(permission.uname(), "");
                assert_eq!(permission.uid(), 1000);
                assert_eq!(permission.gid(), 2000);
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
