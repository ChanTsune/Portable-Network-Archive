#[cfg(not(target_family = "wasm"))]
mod dump;

use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_xattr_set() {
    setup();
    TestResources::extract_in("raw/", "xattr_set/in/").unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_set/xattr_set.pna",
        "--overwrite",
        "xattr_set/in/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_set/xattr_set.pna",
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "xattr_set/in/raw/empty.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "get",
        "xattr_set/xattr_set.pna",
        "xattr_set/in/raw/empty.txt",
        "--name",
        "user.name",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "xattr_set/xattr_set.pna",
        "--overwrite",
        "--out-dir",
        "xattr_set/out/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--strip-components",
        "2",
    ]))
    .unwrap();

    diff("xattr_set/in/", "xattr_set/out/").unwrap();

    #[cfg(all(unix, not(target_os = "netbsd")))]
    if xattr::SUPPORTED_PLATFORM {
        assert_eq!(
            xattr::get("xattr_set/out/raw/empty.txt", "user.name")
                .unwrap()
                .unwrap(),
            b"pna developers!"
        );
    }
}

#[test]
fn archive_xattr_remove() {
    setup();
    TestResources::extract_in("raw/", "xattr_remove/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_remove/xattr_remove.pna",
        "--overwrite",
        "xattr_remove/in/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_remove/xattr_remove.pna",
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "xattr_remove/in/raw/empty.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_remove/xattr_remove.pna",
        "--remove",
        "user.name",
        "xattr_remove/in/raw/empty.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "get",
        "xattr_remove/xattr_remove.pna",
        "xattr_remove/in/raw/empty.txt",
        "--name",
        "user.name",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "xattr_remove/xattr_remove.pna",
        "--overwrite",
        "--out-dir",
        "xattr_remove/out/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--strip-components",
        "2",
    ]))
    .unwrap();

    diff("xattr_remove/in/", "xattr_remove/out/").unwrap();

    #[cfg(all(unix, not(target_os = "netbsd")))]
    if xattr::SUPPORTED_PLATFORM {
        assert!(xattr::get("xattr_remove/out/raw/empty.txt", "user.name")
            .unwrap()
            .is_none());
    }
}
