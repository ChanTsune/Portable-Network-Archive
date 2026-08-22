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
/// Expectation: Every entry's ownership is converted to owner-facet chunks
/// (rescued from fPRM); the legacy fPRM chunk is not emitted; timestamps and
/// entry count are preserved.
#[test]
fn migrate_converts_fprm_to_owner_facet() {
    setup();
    TestResources::extract_in("0.33.0/zstd_keep_all.pna", "migrate_fprm_0_33_0/").unwrap();

    let mut pre: BTreeMap<String, Captured> = BTreeMap::new();
    archive::for_each_entry(LEGACY_FIXTURE, |entry| {
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        // Pin the fixture's known legacy values before capturing them, so a
        // read path that silently drops fPRM (returning None everywhere)
        // fails here instead of passing later by comparing None against None.
        assert_eq!(meta.owner_uid().map(|v| v.get()), Some(0), "uid {path}");
        assert_eq!(meta.owner_gid().map(|v| v.get()), Some(0), "gid {path}");
        assert_eq!(
            meta.owner_user_name().map(|v| v.as_str()),
            Some("root"),
            "uname {path}"
        );
        assert_eq!(
            meta.owner_group_name().map(|v| v.as_str()),
            Some("root"),
            "gname {path}"
        );
        let expected_mode = match entry.header().data_kind() {
            pna::DataKind::DIRECTORY => 0o755,
            pna::DataKind::FILE => 0o644,
            other => panic!("unexpected data kind in fixture: {other:?}"),
        };
        assert_eq!(
            meta.permission_mode().map(|v| v.get()),
            Some(expected_mode),
            "mode {path}"
        );
        pre.insert(
            path,
            Captured {
                uid: meta.owner_uid().map(|v| v.get()),
                gid: meta.owner_gid().map(|v| v.get()),
                uname: meta.owner_user_name().map(|v| v.as_str().to_string()),
                gname: meta.owner_group_name().map(|v| v.as_str().to_string()),
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
            expected.uname.as_deref(),
            "uname {path}"
        );
        assert_eq!(
            meta.owner_group_name().map(|v| v.as_str()),
            expected.gname.as_deref(),
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

    let bytes = std::fs::read("migrate_fprm_0_33_0/migrated.pna").unwrap();
    assert!(
        !bytes.windows(4).any(|w| w == b"fPRM"),
        "migrate must not emit an fPRM chunk"
    );
}
