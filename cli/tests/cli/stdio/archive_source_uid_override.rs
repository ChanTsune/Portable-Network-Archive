use crate::utils::{EmbedExt, TestResources, archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;

/// Precondition: An fPRM-only archive is used as an `@archive` transform source.
/// Action: Run `experimental stdio --create` with a `--uid` ownership override.
/// Expectation: Each transformed entry carries the overridden owner uid as an
/// owner-facet value, and the legacy fPRM chunk is NOT also emitted (no
/// fPRM/owner-facet coexistence).
#[test]
#[allow(deprecated)]
fn stdio_archive_source_uid_override_drops_fprm() {
    setup();

    let base = PathBuf::from("stdio_archive_source_uid_override");
    TestResources::extract_in("zstd_keep_all.pna", &base).unwrap();

    let mut pre_count = 0usize;
    archive::for_each_entry(base.join("zstd_keep_all.pna"), |entry| {
        pre_count += 1;
        assert!(
            entry.metadata().permission().is_some(),
            "fixture entry {} should carry fPRM permission",
            entry.header().path()
        );
    })
    .unwrap();
    assert!(pre_count > 0, "source archive should contain entries");

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
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

    let mut post_count = 0usize;
    archive::for_each_entry(&output_archive, |entry| {
        post_count += 1;
        let path = entry.header().path().to_string();
        let meta = entry.metadata();
        assert!(
            meta.permission().is_none(),
            "fPRM must not coexist with owner-facet after override for {path}"
        );
        assert_eq!(
            meta.owner_uid().map(|v| v.get()),
            Some(4242),
            "overridden uid should be emitted as owner-facet for {path}"
        );
    })
    .unwrap();
    assert_eq!(
        post_count, pre_count,
        "transform should preserve all entries"
    );
}
