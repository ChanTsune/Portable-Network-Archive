use crate::utils::{setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, path::Path};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn stdio_no_same_permissions() {
    setup();
    TestResources::extract_in("raw/", "stdio_no_same_permissions/input/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "stdio_no_same_permissions/input/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "stdio_no_same_permissions/archive.pna",
        "--overwrite",
        "stdio_no_same_permissions/input/",
        "-p",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(
        fs::exists("stdio_no_same_permissions/archive.pna").unwrap(),
        "archive should be created",
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "stdio",
        "-x",
        "-f",
        "stdio_no_same_permissions/archive.pna",
        "--overwrite",
        "-p",
        "--out-dir",
        "stdio_no_same_permissions/out_keep/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "stdio",
        "-x",
        "-f",
        "stdio_no_same_permissions/archive.pna",
        "--overwrite",
        "--no-same-permissions",
        "--out-dir",
        "stdio_no_same_permissions/out_default/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let keep_file = Path::new(
        "stdio_no_same_permissions/out_keep/stdio_no_same_permissions/input/raw/text.txt",
    );
    let default_file = Path::new(
        "stdio_no_same_permissions/out_default/stdio_no_same_permissions/input/raw/text.txt",
    );

    assert!(keep_file.exists(), "extraction with -p should create file");
    assert!(
        default_file.exists(),
        "extraction with --no-same-permissions should create file"
    );

    let original = fs::read("stdio_no_same_permissions/input/raw/text.txt").unwrap();
    assert_eq!(original, fs::read(keep_file).unwrap());
    assert_eq!(original, fs::read(default_file).unwrap());

    #[cfg(unix)]
    {
        let kept = fs::symlink_metadata(keep_file).unwrap();
        let default = fs::symlink_metadata(default_file).unwrap();
        assert_eq!(kept.permissions().mode() & 0o777, 0o777);
        assert_ne!(default.permissions().mode() & 0o777, 0o777);
    }
}
