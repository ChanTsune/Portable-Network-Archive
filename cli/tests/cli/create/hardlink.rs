use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, path::Path};

fn init_resource<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    let _ = fs::remove_dir_all(path);
    fs::create_dir_all(path).unwrap();
    fs::write(path.join("origin1.txt"), b"original text\n").unwrap();
    fs::hard_link(path.join("origin1.txt"), path.join("linked1.txt")).unwrap();
}

#[test]
fn create_hardlink() {
    setup();
    init_resource("create_hardlink/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_hardlink/create_hardlink.pna",
        "--overwrite",
        "--keep-dir",
        "create_hardlink/source",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // NOTE: The same file that is encountered the second time or later becomes a hard link in the archive.
    let mut hard_link_count = 0;
    archive::for_each_entry("create_hardlink/create_hardlink.pna", |entry| {
        match entry.header().path().as_str() {
            "create_hardlink/source" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "create_hardlink/source/linked1.txt" | "create_hardlink/source/origin1.txt" => {
                if entry.header().data_kind() == pna::DataKind::HardLink {
                    hard_link_count += 1;
                }
            }
            p => panic!("Unexpected entry: {p}"),
        }
    })
    .unwrap();
    assert_eq!(hard_link_count, 1);
}

#[test]
fn create_hardlink_hard_dereference() {
    setup();
    init_resource("create_hardlink_hard_dereference/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_hardlink_hard_dereference/create_hardlink.pna",
        "--overwrite",
        "--keep-dir",
        "--hard-dereference",
        "create_hardlink_hard_dereference/source",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(
        "create_hardlink_hard_dereference/create_hardlink.pna",
        |entry| match entry.header().path().as_str() {
            "create_hardlink_hard_dereference/source" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "create_hardlink_hard_dereference/source/linked1.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "create_hardlink_hard_dereference/source/origin1.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            p => panic!("Unexpected entry: {p}"),
        },
    )
    .unwrap();
}
