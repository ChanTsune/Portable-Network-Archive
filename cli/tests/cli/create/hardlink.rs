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
    let mut file_count = 0;
    let mut hard_link_count = 0;
    archive::for_each_entry("create_hardlink/create_hardlink.pna", |entry| {
        match entry.header().path().as_str() {
            "create_hardlink/source" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory);
            }
            "create_hardlink/source/linked1.txt" | "create_hardlink/source/origin1.txt" => {
                match entry.header().data_kind() {
                    pna::DataKind::File => file_count += 1,
                    pna::DataKind::HardLink => hard_link_count += 1,
                    kind => panic!("Unexpected data kind {:?} for file entry", kind),
                }
            }
            p => panic!("Unexpected entry: {p}"),
        }
    })
    .unwrap();
    #[cfg(not(target_family = "wasm"))]
    {
        assert_eq!(file_count, 1);
        assert_eq!(hard_link_count, 1);
    };
    #[cfg(target_family = "wasm")]
    {
        // Wasm not supported hardlink detection
        assert_eq!(file_count, 2);
        assert_eq!(hard_link_count, 0);
    };
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
