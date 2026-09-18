use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive contains multiple .txt files with permission 0o644.
/// Action: Run `pna experimental chmod` with glob pattern `**/*.txt` and mode `755`.
/// Expectation: All .txt files have permission 0o755 in the archive; other files are unchanged.
#[test]
fn chmod_glob_pattern_txt_files() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_glob_txt.pna",
        &[
            FileEntryDef {
                path: "dir/text.txt",
                content: b"text content",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/empty.txt",
                content: b"",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/sub/child.txt",
                content: b"child content",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/images/icon.png",
                content: b"png data",
                permission: 0o600,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "--overwrite",
        "-f",
        "chmod_glob_txt.pna",
        "755",
        "**/*.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::modes_by_entry("chmod_glob_txt.pna"),
        vec![
            ("dir/text.txt".to_string(), Some(0o755)),
            ("dir/empty.txt".to_string(), Some(0o755)),
            ("dir/sub/child.txt".to_string(), Some(0o755)),
            ("dir/images/icon.png".to_string(), Some(0o600)),
        ]
    );
}

/// Precondition: An archive contains files in nested directories.
/// Action: Run `pna experimental chmod` with pattern targeting a subdirectory.
/// Expectation: Only files in the matching subdirectory have changed permissions in the archive.
#[test]
fn chmod_glob_pattern_subdirectory() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_glob_subdir.pna",
        &[
            FileEntryDef {
                path: "dir/images/icon.png",
                content: b"png data",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/images/icon.svg",
                content: b"svg data",
                permission: 0o644,
            },
            FileEntryDef {
                path: "dir/text.txt",
                content: b"text content",
                permission: 0o644,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "--overwrite",
        "-f",
        "chmod_glob_subdir.pna",
        "755",
        "**/images/*",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::modes_by_entry("chmod_glob_subdir.pna"),
        vec![
            ("dir/images/icon.png".to_string(), Some(0o755)),
            ("dir/images/icon.svg".to_string(), Some(0o755)),
            ("dir/text.txt".to_string(), Some(0o644)),
        ]
    );
}
