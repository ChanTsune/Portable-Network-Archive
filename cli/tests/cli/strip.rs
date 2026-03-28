use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::Duration;
use portable_network_archive::cli;
use std::collections::BTreeMap;

/// Precondition: An archive contains entries with timestamp and permission metadata.
/// Action: Run `pna strip` keeping only timestamps.
/// Expectation: Timestamps are preserved with their original values; permissions are removed.
#[test]
fn strip_removes_unspecified_metadata() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "strip_metadata/").unwrap();

    // Record metadata before strip
    let mut pre_timestamps: BTreeMap<String, Option<Duration>> = BTreeMap::new();
    archive::for_each_entry("strip_metadata/zstd_keep_all.pna", |entry| {
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        assert!(
            meta.permission().is_some() && meta.modified().is_some(),
            "entry {path} should have both permission and timestamp before strip"
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
    })
    .unwrap();
    assert_eq!(
        post_count,
        pre_timestamps.len(),
        "strip should preserve all entries"
    );
}
