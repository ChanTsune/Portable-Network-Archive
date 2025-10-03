use crate::utils::{setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs::{self, File},
    time::{Duration, SystemTime},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn stdio_modification_time_flag() {
    setup();
    TestResources::extract_in("raw/", "stdio_modification_time/input/").unwrap();

    let source = "stdio_modification_time/input/raw/text.txt";
    #[cfg(unix)]
    fs::set_permissions(source, fs::Permissions::from_mode(0o600)).unwrap();
    let old_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
    File::options()
        .write(true)
        .open(source)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(old_mtime))
        .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "stdio",
        "-c",
        "-f",
        "stdio_modification_time/archive.pna",
        "--overwrite",
        "-p",
        "stdio_modification_time/input/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extraction with default behaviour should keep archived mtime when -p is supplied.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "stdio",
        "-x",
        "-f",
        "stdio_modification_time/archive.pna",
        "--overwrite",
        "-p",
        "--out-dir",
        "stdio_modification_time/out_keep/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extraction with -m should touch modification time instead of restoring.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "stdio",
        "-x",
        "-f",
        "stdio_modification_time/archive.pna",
        "--overwrite",
        "-m",
        "--out-dir",
        "stdio_modification_time/out_now/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let keep_path = "stdio_modification_time/out_keep/stdio_modification_time/input/raw/text.txt";
    let now_path = "stdio_modification_time/out_now/stdio_modification_time/input/raw/text.txt";
    assert!(fs::exists(keep_path).unwrap());
    assert!(fs::exists(now_path).unwrap());

    let keep_meta = fs::symlink_metadata(keep_path).unwrap();
    let now_meta = fs::symlink_metadata(now_path).unwrap();
    let keep_diff = keep_meta
        .modified()
        .unwrap()
        .duration_since(old_mtime)
        .unwrap_or(Duration::ZERO);
    assert!(keep_diff <= Duration::from_secs(1));
    assert!(now_meta.modified().unwrap() >= SystemTime::now() - Duration::from_secs(5));
}
