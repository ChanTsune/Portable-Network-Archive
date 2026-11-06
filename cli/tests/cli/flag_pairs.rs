use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;
use std::fs;

/// Every flag that has a `--no-` counterpart, per command that exposes the pair.
const PAIRS: &[(&[&str], &str, &str)] = &[
    (&["c", "-f", "a.pna"], "--recursive", "--no-recursive"),
    (&["c", "-f", "a.pna"], "--keep-dir", "--no-keep-dir"),
    (&["c", "-f", "a.pna"], "--overwrite", "--no-overwrite"),
    (
        &["c", "-f", "a.pna"],
        "--preserve-xattrs",
        "--no-preserve-xattrs",
    ),
    (
        &["c", "-f", "a.pna", "--unstable"],
        "--preserve-permissions",
        "--no-preserve-permissions",
    ),
    (
        &["c", "-f", "a.pna", "--unstable"],
        "--preserve-acls",
        "--no-preserve-acls",
    ),
    (&["append", "-f", "a.pna"], "--recursive", "--no-recursive"),
    (&["append", "-f", "a.pna"], "--keep-dir", "--no-keep-dir"),
    (
        &["append", "-f", "a.pna"],
        "--preserve-xattrs",
        "--no-preserve-xattrs",
    ),
    (
        &["append", "-f", "a.pna", "--unstable"],
        "--preserve-permissions",
        "--no-preserve-permissions",
    ),
    (
        &["append", "-f", "a.pna", "--unstable"],
        "--preserve-acls",
        "--no-preserve-acls",
    ),
    (
        &["experimental", "update", "-f", "a.pna"],
        "--recursive",
        "--no-recursive",
    ),
    (
        &["experimental", "update", "-f", "a.pna"],
        "--keep-dir",
        "--no-keep-dir",
    ),
    (
        &["experimental", "update", "-f", "a.pna"],
        "--preserve-xattrs",
        "--no-preserve-xattrs",
    ),
    (
        &["experimental", "update", "-f", "a.pna", "--unstable"],
        "--preserve-permissions",
        "--no-preserve-permissions",
    ),
    (
        &["experimental", "update", "-f", "a.pna", "--unstable"],
        "--preserve-acls",
        "--no-preserve-acls",
    ),
    (
        &["x", "-f", "a.pna"],
        "--preserve-xattrs",
        "--no-preserve-xattrs",
    ),
    (
        &["x", "-f", "a.pna", "--unstable"],
        "--preserve-permissions",
        "--no-preserve-permissions",
    ),
    (
        &["x", "-f", "a.pna", "--unstable"],
        "--preserve-acls",
        "--no-preserve-acls",
    ),
    (&["x", "-f", "a.pna"], "--same-owner", "--no-same-owner"),
    (
        &["x", "-f", "a.pna", "--unstable"],
        "--safe-writes",
        "--no-safe-writes",
    ),
    (
        &["x", "-f", "a.pna", "--unstable"],
        "--allow-unsafe-links",
        "--no-allow-unsafe-links",
    ),
    (&["list", "-f", "a.pna"], "--recursive", "--no-recursive"),
    (&["split", "-f", "a.pna"], "--overwrite", "--no-overwrite"),
    (&["concat", "-f", "a.pna"], "--overwrite", "--no-overwrite"),
    (
        &["experimental", "chown", "-f", "a.pna", "owner"],
        "--owner-lookup",
        "--no-owner-lookup",
    ),
    (
        &["strip", "--output", "out.pna"],
        "--overwrite",
        "--no-overwrite",
    ),
    (
        &["delete", "present.txt", "--output", "out.pna"],
        "--overwrite",
        "--no-overwrite",
    ),
    (
        &[
            "experimental",
            "chmod",
            "600",
            "present.txt",
            "--output",
            "out.pna",
        ],
        "--overwrite",
        "--no-overwrite",
    ),
    (
        &[
            "experimental",
            "chown",
            "new_user",
            "present.txt",
            "--no-owner-lookup",
            "--output",
            "out.pna",
        ],
        "--overwrite",
        "--no-overwrite",
    ),
    (
        &[
            "experimental",
            "acl",
            "set",
            "present.txt",
            "--modify",
            "u:test:r",
            "--output",
            "out.pna",
        ],
        "--overwrite",
        "--no-overwrite",
    ),
    (
        &[
            "xattr",
            "set",
            "present.txt",
            "--name",
            "user.test",
            "--value",
            "value",
            "--output",
            "out.pna",
        ],
        "--overwrite",
        "--no-overwrite",
    ),
    (
        &["migrate", "--output", "out.pna"],
        "--overwrite",
        "--no-overwrite",
    ),
    (
        &["sort", "--output", "out.pna"],
        "--overwrite",
        "--no-overwrite",
    ),
];

const DEPRECATED_NO_OVERWRITE_COMMANDS: &[&[&str]] = &[
    &["strip"],
    &["delete", "present.txt"],
    &["experimental", "chmod", "600", "present.txt"],
    &[
        "experimental",
        "chown",
        "new_user",
        "present.txt",
        "--no-owner-lookup",
    ],
    &[
        "experimental",
        "acl",
        "set",
        "present.txt",
        "--modify",
        "u:test:r",
    ],
    &[
        "xattr",
        "set",
        "present.txt",
        "--name",
        "user.test",
        "--value",
        "value",
    ],
    &["migrate"],
    &["sort"],
];

/// Precondition: A native command exposes a flag together with its `--no-` counterpart.
/// Action: Pass both spellings in one invocation, in either order.
/// Expectation: Parsing reports the combination as ambiguous instead of picking one.
#[test]
fn native_commands_reject_a_flag_with_its_negation() {
    for (base, yes, no) in PAIRS {
        for pair in [[yes, no], [no, yes]] {
            let argv = std::iter::once(&"pna")
                .chain(base.iter())
                .chain(pair)
                .copied()
                .collect::<Vec<_>>();
            let err = cli::Cli::try_parse_from(&argv)
                .err()
                .unwrap_or_else(|| panic!("accepted {argv:?}"));
            assert!(
                err.to_string().contains("cannot be used with"),
                "{argv:?} was rejected for another reason: {err}"
            );
        }
    }
}

/// Precondition: A native command exposes a flag with a `--no-` counterpart.
/// Action: Pass each spelling on its own.
/// Expectation: Both are accepted; deprecated no-overwrite warns and preserves existing outputs.
#[test]
fn native_commands_accept_either_spelling_alone() {
    for (base, yes, no) in PAIRS {
        for single in [yes, no] {
            let argv = std::iter::once(&"pna")
                .chain(base.iter())
                .chain(std::iter::once(single))
                .copied()
                .collect::<Vec<_>>();
            cli::Cli::try_parse_from(&argv).unwrap_or_else(|e| panic!("rejected {argv:?}: {e}"));
        }
    }

    setup();
    let destination = "native_no_overwrite.pna";
    let mut archive = pna::Archive::write_header(Vec::new()).unwrap();
    archive
        .add_entry(
            pna::FileEntryBuilder::new("present.txt".into())
                .unwrap()
                .build()
                .unwrap(),
        )
        .unwrap();
    let input = archive.finalize().unwrap();
    fs::write(destination, b"original destination").unwrap();
    for args in DEPRECATED_NO_OVERWRITE_COMMANDS {
        cargo_bin_cmd!("pna")
            .args(*args)
            .args(["--no-overwrite", "--output", destination])
            .write_stdin(input.clone())
            .assert()
            .failure()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("`--no-overwrite` is deprecated and will be removed in a future release; omit it to preserve existing output archives."))
            .stderr(predicate::str::contains("already exists"));
        assert_eq!(fs::read(destination).unwrap(), b"original destination");
    }
}
