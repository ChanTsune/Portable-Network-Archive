use crate::utils::{archive, archive::FileEntryDef, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

/// Precondition: A stored archive with one file entry exists.
/// Action: Run `experimental chmod` without `--output` (no `--quiet`, so warnings stay visible).
/// Expectation: Succeeds, the archive is rewritten in place, and a deprecation
/// warning names the rewritten path.
#[test]
fn chmod_without_output_warns_but_rewrites_in_place() {
    setup();
    archive::create_archive_with_permissions(
        "chmod_omitted_output_warn.pna",
        &[FileEntryDef {
            path: "test.txt",
            content: b"test content",
            permission: 0o777,
        }],
    )
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chmod",
            "-f",
            "chmod_omitted_output_warn.pna",
            "--",
            "-x",
            "test.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "omitting `--output` is deprecated and will write to standard output instead of rewriting",
        ))
        .stderr(predicate::str::contains(
            "chmod_omitted_output_warn.pna",
        ));

    assert_eq!(
        archive::modes_by_entry("chmod_omitted_output_warn.pna"),
        vec![("test.txt".to_string(), Some(0o666))]
    );
}
