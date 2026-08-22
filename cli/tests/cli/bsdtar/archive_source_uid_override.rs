use crate::utils::{EmbedExt, TestResources, archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;

/// Precondition: An owner-facet archive is used as an `@archive` transform source.
/// Action: Run `compat bsdtar --create` with a `--uid` ownership override.
/// Expectation: Each transformed entry carries the overridden owner uid as an
/// owner-facet value while preserving the other owner facets, and the legacy
/// fPRM chunk is not emitted.
#[test]
fn bsdtar_archive_source_uid_override_drops_fprm() {
    setup();

    let base = PathBuf::from("bsdtar_archive_source_uid_override");
    TestResources::extract_in("zstd_keep_all.pna", &base).unwrap();
    let source_archive = base.join("zstd_keep_all.pna");

    let mut pre = Vec::new();
    archive::for_each_entry(&source_archive, |entry| {
        let meta = entry.metadata();
        assert!(
            meta.owner_uid().is_some() && meta.permission_mode().is_some(),
            "fixture entry {} should carry owner facets",
            entry.header().path()
        );
        pre.push((
            entry.header().path().to_string(),
            meta.owner_gid().map(|v| v.get()),
            meta.owner_user_name().map(|v| v.as_str().to_owned()),
            meta.owner_group_name().map(|v| v.as_str().to_owned()),
            meta.permission_mode().map(|v| v.get()),
        ));
    })
    .unwrap();
    assert!(!pre.is_empty(), "source archive should contain entries");
    let source_bytes = std::fs::read(&source_archive).unwrap();
    assert!(
        !source_bytes.windows(4).any(|w| w == b"fPRM"),
        "fixture archive should not carry an fPRM chunk"
    );

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@zstd_keep_all.pna",
            "--uid",
            "4242",
        ])
        .assert()
        .success();

    let mut post = Vec::new();
    archive::for_each_entry(&output_archive, |entry| {
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        assert_eq!(
            meta.owner_uid().map(|v| v.get()),
            Some(4242),
            "overridden uid should be emitted as owner-facet for {path}"
        );
        post.push((
            path,
            meta.owner_gid().map(|v| v.get()),
            meta.owner_user_name().map(|v| v.as_str().to_owned()),
            meta.owner_group_name().map(|v| v.as_str().to_owned()),
            meta.permission_mode().map(|v| v.get()),
        ));
    })
    .unwrap();
    assert_eq!(
        post.len(),
        pre.len(),
        "transform should preserve all entries"
    );
    assert_eq!(
        post, pre,
        "uid override should preserve non-uid owner facets"
    );
    let output_bytes = std::fs::read(&output_archive).unwrap();
    assert!(
        !output_bytes.windows(4).any(|w| w == b"fPRM"),
        "fPRM must not be emitted after owner override"
    );
}
