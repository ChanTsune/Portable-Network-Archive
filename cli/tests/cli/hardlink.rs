use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryBuilder, WriteOptions};
use portable_network_archive::{cli, command::Command};
use std::{fs, io::Write, path::Path};

fn init_resource<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file("origin1.txt".into(), WriteOptions::builder().build())
                    .unwrap();
            builder.write_all(b"original text\n").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder =
                EntryBuilder::new_hard_link("linked1.txt".into(), "origin1.txt".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder =
                EntryBuilder::new_hard_link("dir/linked1.txt".into(), "../origin1.txt".into())
                    .unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file("dir/origin2.txt".into(), WriteOptions::builder().build())
                    .unwrap();
            builder.write_all(b"original text text\n").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder =
                EntryBuilder::new_hard_link("dir/linked2.txt".into(), "origin2.txt".into())
                    .unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder =
                EntryBuilder::new_hard_link("linked2.txt".into(), "dir/origin2.txt".into())
                    .unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

#[test]
fn hardlink() {
    setup();
    init_resource("hardlink/hardlink.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "hardlink/hardlink.pna",
        "--overwrite",
        "--out-dir",
        "hardlink/dist",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        "original text\n",
        fs::read_to_string("hardlink/dist/linked1.txt",).unwrap()
    );

    // Check skipped extract unsafe link
    assert!(!fs::try_exists("hardlink/dist/dir/linked1.txt").unwrap());

    assert_eq!(
        "original text text\n",
        fs::read_to_string("hardlink/dist/dir/linked2.txt",).unwrap()
    );
    assert_eq!(
        "original text text\n",
        fs::read_to_string("hardlink/dist/linked2.txt",).unwrap()
    );
}

#[test]
fn hardlink_allow_unsafe_links() {
    setup();
    init_resource("hardlink_allow_unsafe_links/hardlink.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "hardlink_allow_unsafe_links/hardlink.pna",
        "--allow-unsafe-links",
        "--overwrite",
        "--out-dir",
        "hardlink_allow_unsafe_links/dist",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        "original text\n",
        fs::read_to_string("hardlink_allow_unsafe_links/dist/linked1.txt",).unwrap()
    );
    assert_eq!(
        "original text\n",
        fs::read_to_string("hardlink_allow_unsafe_links/dist/dir/linked1.txt",).unwrap()
    );

    assert_eq!(
        "original text text\n",
        fs::read_to_string("hardlink_allow_unsafe_links/dist/dir/linked2.txt",).unwrap()
    );
    assert_eq!(
        "original text text\n",
        fs::read_to_string("hardlink_allow_unsafe_links/dist/linked2.txt",).unwrap()
    );
}
