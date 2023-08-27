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
}

#[test]
fn symlink() {
    init_resource("../out/symlink/source");
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "../out/symlink.pna",
        "--overwrite",
        "-r",
        "../out/symlink/source",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../out/symlink.pna",
        "--overwrite",
        "--out-dir",
        "../out/symlink/dist",
    ]))
    .unwrap();
}
