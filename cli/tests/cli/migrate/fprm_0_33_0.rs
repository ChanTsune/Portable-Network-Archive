use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::collections::BTreeMap;

const LEGACY_FIXTURE: &str = "migrate_fprm_0_33_0/0.33.0/zstd_keep_all.pna";

struct Captured {
    uid: u64,
    gid: u64,
    uname: String,
    gname: String,
    mode: u16,
    modified: Option<Duration>,
}

/// Precondition: An fPRM-only archive carries ownership and timestamp metadata.
/// Action: Run `pna migrate` to a new output archive.
/// Expectation: Every entry's ownership is preserved as owner-facet chunks
/// (the reader already fills them from fPRM on the way in); timestamps and
/// entry count are preserved.
#[test]
fn migrate_preserves_ownership_from_legacy_fprm_source() {
    setup();
    TestResources::extract_in("0.33.0/zstd_keep_all.pna", "migrate_fprm_0_33_0/").unwrap();

    let mut pre: BTreeMap<String, Captured> = BTreeMap::new();
    archive::for_each_entry(LEGACY_FIXTURE, |entry| {
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        // The reader already folded the fixture's fPRM chunk into the owner
        // facets, so the pre-migrate baseline is read from them directly.
        pre.insert(
            path,
            Captured {
                uid: meta
                    .owner_uid()
                    .expect("fixture entry should carry an owner uid")
                    .get(),
                gid: meta
                    .owner_gid()
                    .expect("fixture entry should carry an owner gid")
                    .get(),
                uname: meta
                    .owner_user_name()
                    .expect("fixture entry should carry an owner user name")
                    .as_str()
                    .to_owned(),
                gname: meta
                    .owner_group_name()
                    .expect("fixture entry should carry an owner group name")
                    .as_str()
                    .to_owned(),
                mode: meta
                    .permission_mode()
                    .expect("fixture entry should carry a permission mode")
                    .get(),
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
            Some(expected.uid),
            "uid {path}"
        );
        assert_eq!(
            meta.owner_gid().map(|v| v.get()),
            Some(expected.gid),
            "gid {path}"
        );
        let expected_uname = if expected.uname.is_empty() {
            None
        } else {
            Some(expected.uname.as_str())
        };
        let expected_gname = if expected.gname.is_empty() {
            None
        } else {
            Some(expected.gname.as_str())
        };
        assert_eq!(
            meta.owner_user_name().map(|v| v.as_str()),
            expected_uname,
            "uname {path}"
        );
        assert_eq!(
            meta.owner_group_name().map(|v| v.as_str()),
            expected_gname,
            "gname {path}"
        );
        assert_eq!(
            meta.permission_mode().map(|v| v.get()),
            Some(expected.mode),
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
