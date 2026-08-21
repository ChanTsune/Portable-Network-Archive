use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::collections::BTreeMap;

const LEGACY_FIXTURE: &str = "migrate_fprm_0_33_0/0.33.0/zstd_keep_all.pna";

struct Captured {
    uid: Option<u64>,
    gid: Option<u64>,
    uname: Option<String>,
    gname: Option<String>,
    mode: Option<u16>,
    modified: Option<Duration>,
}

/// Precondition: An fPRM-only archive carries ownership and timestamp metadata.
/// Action: Run `pna migrate` to a new output archive.
/// Expectation: Every entry's ownership survives as owner-facet chunks, as do
/// timestamps and the entry count.
#[test]
fn migrate_converts_fprm_to_owner_facet() {
    setup();
    TestResources::extract_in("0.33.0/zstd_keep_all.pna", "migrate_fprm_0_33_0/").unwrap();

    let mut pre: BTreeMap<String, Captured> = BTreeMap::new();
    archive::for_each_entry(LEGACY_FIXTURE, |entry| {
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        pre.insert(
            path,
            Captured {
                uid: meta.owner_uid().map(|v| v.get()),
                gid: meta.owner_gid().map(|v| v.get()),
                uname: meta.owner_user_name().map(|v| v.as_str().to_owned()),
                gname: meta.owner_group_name().map(|v| v.as_str().to_owned()),
                mode: meta.permission_mode().map(|v| v.get()),
                modified: meta.modified(),
            },
        );
    })
    .unwrap();
    assert!(!pre.is_empty(), "archive should contain entries");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "migrate",
        "-f",
        LEGACY_FIXTURE,
        "--output",
        "migrate_fprm_0_33_0/migrated.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut post_count = 0usize;
    archive::for_each_entry("migrate_fprm_0_33_0/migrated.pna", |entry| {
        post_count += 1;
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        let expected = pre
            .get(&path)
            .unwrap_or_else(|| panic!("unexpected entry after migrate: {path}"));
        assert_eq!(
            meta.owner_uid().map(|v| v.get()),
            expected.uid,
            "uid {path}"
        );
        assert_eq!(
            meta.owner_gid().map(|v| v.get()),
            expected.gid,
            "gid {path}"
        );
        assert_eq!(
            meta.owner_user_name().map(|v| v.as_str()),
            expected.uname.as_deref().filter(|v| !v.is_empty()),
            "uname {path}"
        );
        assert_eq!(
            meta.owner_group_name().map(|v| v.as_str()),
            expected.gname.as_deref().filter(|v| !v.is_empty()),
            "gname {path}"
        );
        assert_eq!(
            meta.permission_mode().map(|v| v.get()),
            expected.mode,
            "mode {path}"
        );
        assert_eq!(
            meta.modified(),
            expected.modified,
            "modified timestamp {path}"
        );
    })
    .unwrap();
    assert_eq!(post_count, pre.len(), "migrate should preserve all entries");
}
