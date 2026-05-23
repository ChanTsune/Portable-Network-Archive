use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::collections::BTreeMap;

/// Precondition: An archive contains entries with timestamp and permission metadata.
/// Action: Run `pna strip` keeping only timestamps.
/// Expectation: Timestamps are preserved with their original values; permissions are removed.
#[test]
#[allow(deprecated)]
fn strip_removes_unspecified_metadata() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "strip_metadata/").unwrap();

    // Record metadata before strip
    let mut pre_timestamps: BTreeMap<String, Option<Duration>> = BTreeMap::new();
    archive::for_each_entry("strip_metadata/zstd_keep_all.pna", |entry| {
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        assert!(
            meta.permission_mode().is_some() && meta.modified().is_some(),
            "entry {path} should have both permission mode and timestamp before strip"
        );
        pre_timestamps.insert(path, meta.modified());
    })
    .unwrap();
    assert!(!pre_timestamps.is_empty(), "archive should contain entries");

    // Strip keeping only timestamps
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "-f",
        "strip_metadata/zstd_keep_all.pna",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify: timestamps preserved with original values, permissions removed
    let mut post_count = 0usize;
    archive::for_each_entry("strip_metadata/zstd_keep_all.pna", |entry| {
        post_count += 1;
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        assert_eq!(
            meta.modified(),
            *pre_timestamps
                .get(&path)
                .unwrap_or_else(|| { panic!("unexpected entry after strip: {path}") }),
            "timestamp value should be preserved for {path}"
        );
        assert!(
            meta.permission().is_none(),
            "permissions should be removed for {path}"
        );
        assert!(
            meta.owner_uid().is_none()
                && meta.owner_gid().is_none()
                && meta.owner_user_name().is_none()
                && meta.owner_group_name().is_none()
                && meta.permission_mode().is_none()
                && meta.owner_user_sid().is_none()
                && meta.owner_group_sid().is_none(),
            "owner facets should also be removed for {path}"
        );
    })
    .unwrap();
    assert_eq!(
        post_count,
        pre_timestamps.len(),
        "strip should preserve all entries"
    );
}

/// Precondition: An archive carries ownership metadata as owner facets.
/// Action: Run `pna strip --keep-permission`.
/// Expectation: Ownership is preserved as owner-facet chunks; the legacy fPRM chunk is not emitted.
#[test]
#[allow(deprecated)]
fn strip_keep_permission_preserves_owner_facets() {
    struct OwnerFacets {
        uid: Option<u64>,
        gid: Option<u64>,
        uname: Option<String>,
        gname: Option<String>,
        mode: Option<u16>,
    }

    setup();
    TestResources::extract_in("zstd_keep_all.pna", "strip_keep_perm/").unwrap();

    let mut pre: BTreeMap<String, OwnerFacets> = BTreeMap::new();
    archive::for_each_entry("strip_keep_perm/zstd_keep_all.pna", |entry| {
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        assert!(
            meta.permission().is_none(),
            "fixture entry should not carry fPRM permission for {path}"
        );
        assert!(
            meta.permission_mode().is_some(),
            "fixture entry should carry owner-facet permission mode for {path}"
        );
        pre.insert(
            path,
            OwnerFacets {
                uid: meta.owner_uid().map(|v| v.get()),
                gid: meta.owner_gid().map(|v| v.get()),
                uname: meta.owner_user_name().map(|v| v.as_str().to_owned()),
                gname: meta.owner_group_name().map(|v| v.as_str().to_owned()),
                mode: meta.permission_mode().map(|v| v.get()),
            },
        );
    })
    .unwrap();
    assert!(!pre.is_empty(), "archive should contain entries");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "-f",
        "strip_keep_perm/zstd_keep_all.pna",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut post_count = 0usize;
    archive::for_each_entry("strip_keep_perm/zstd_keep_all.pna", |entry| {
        post_count += 1;
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        let expected = pre
            .get(&path)
            .unwrap_or_else(|| panic!("unexpected entry after strip: {path}"));
        assert!(
            meta.permission().is_none(),
            "fPRM must not be emitted after strip --keep-permission for {path}"
        );
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
    })
    .unwrap();
    assert_eq!(post_count, pre.len(), "strip should preserve all entries");
}
