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

    archive::for_each_entry("chmod_glob_txt.pna", |entry| {
        let path = entry.header().path();
        let path_str = path.as_str();
        if let Some(pm) = entry.metadata().permission_mode() {
            if path_str.ends_with(".txt") {
                assert_eq!(pm.get() & 0o777, 0o755, "{} should be 755", path_str);
            } else if path_str.ends_with(".png") {
                assert_eq!(pm.get() & 0o777, 0o600, "icon.png should remain 600");
            }
        }
    })
    .unwrap();
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

    archive::for_each_entry("chmod_glob_subdir.pna", |entry| {
        let path = entry.header().path();
        let path_str = path.as_str();
        if let Some(pm) = entry.metadata().permission_mode() {
            if path_str.contains("/images/") {
                assert_eq!(pm.get() & 0o777, 0o755, "{} should be 755", path_str);
            } else if path_str.ends_with(".txt") {
                assert_eq!(pm.get() & 0o777, 0o644, "text.txt should remain 644");
            }
        }
    })
    .unwrap();
}
