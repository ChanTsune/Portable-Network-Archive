#![cfg(not(target_family = "wasm"))]
use crate::utils::{EmbedExt, TestResources, archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::path::Path;

/// Precondition: An archive with multiple entries exists.
/// Action: Run `xattr set` without `--output` (no `--quiet`, so warnings stay visible).
/// Expectation: Succeeds, the archive is rewritten in place, and a deprecation
/// warning names the rewritten path.
#[test]
fn xattr_set_without_output_warns_but_rewrites_in_place() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_set_warn/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "xattr",
            "set",
            "-f",
            "xattr_set_warn/zstd.pna",
            "--name",
            "user.name",
            "--value",
            "pna developers!",
            "raw/empty.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "omitting `--output` is deprecated and will write to standard output instead of rewriting",
        ))
        .stderr(predicate::str::contains(
            Path::new("xattr_set_warn").join("zstd.pna").display().to_string(),
        ));

    assert_eq!(
        archive::xattrs_by_entry("xattr_set_warn/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.name", b"pna developers!")]
        )]
    );
}
