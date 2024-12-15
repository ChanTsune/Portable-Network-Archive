use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};
use std::io::Write;
use std::path::Path;
use std::{fs, os};

fn init_resource<P: AsRef<Path>>(dir: P) {
    let dir = dir.as_ref();
    if dir.exists() {
        fs::remove_dir_all(dir).unwrap();
    }
    fs::create_dir_all(dir).unwrap();
    let mut file = fs::File::create(dir.join("text.txt")).unwrap();
    file.write_all(b"content").unwrap();
    #[cfg(unix)]
    os::unix::fs::symlink(Path::new("text.txt"), dir.join("link.txt")).unwrap();
    #[cfg(windows)]
    os::windows::fs::symlink_file(Path::new("text.txt"), dir.join("link.txt")).unwrap();
    #[cfg(target_os = "wasi")]
    os::wasi::fs::symlink_path(Path::new("text.txt"), dir.join("link.txt")).unwrap();
}

#[test]
fn symlink() {
    setup();
    init_resource(format!("{}/symlink/source", env!("CARGO_TARGET_TMPDIR")));
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/symlink.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        &format!("{}/symlink/source", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/symlink.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/symlink/dist", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}
