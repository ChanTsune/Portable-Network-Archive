use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::collections::HashMap;

#[test]
fn chown_no_owner_lookup_numeric() {
    setup();
    TestResources::extract_in("raw/", "chown_no_owner_lookup_numeric/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
            (
                entry.metadata().owner_uid().map(|v| v.get()),
                entry.metadata().owner_gid().map(|v| v.get()),
                entry
                    .metadata()
                    .owner_user_name()
                    .map(|v| v.as_str().to_string()),
                entry
                    .metadata()
                    .owner_group_name()
                    .map(|v| v.as_str().to_string()),
                entry.metadata().permission_mode().map(|v| v.get()),
            ),
        );
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
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
                let original = original_owners.get(path).unwrap();
                assert!(entry.metadata().owner_group_name().is_none());
                assert!(entry.metadata().owner_user_name().is_none());
                assert_eq!(entry.metadata().owner_uid().unwrap().get(), 1000);
                assert_eq!(entry.metadata().owner_gid().unwrap().get(), 2000);
                assert_eq!(
                    entry.metadata().permission_mode().map(|v| v.get()),
                    original.4
                );
            }
            path => {
                let original = original_owners.get(path).unwrap();
                let actual = (
                    entry.metadata().owner_uid().map(|v| v.get()),
                    entry.metadata().owner_gid().map(|v| v.get()),
                    entry
                        .metadata()
                        .owner_user_name()
                        .map(|v| v.as_str().to_string()),
                    entry
                        .metadata()
                        .owner_group_name()
                        .map(|v| v.as_str().to_string()),
                    entry.metadata().permission_mode().map(|v| v.get()),
                );
                assert_eq!(&actual, original);
            }
        },
    )
    .unwrap();
}
