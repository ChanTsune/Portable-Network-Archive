use clap::Parser;
use pna::{Archive, EntryBuilder, EntryName, EntryReference, WriteOption};
use portable_network_archive::{cli, command};
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
            let mut builder = EntryBuilder::new_file(
                EntryName::from_lossy("origin1.txt"),
                WriteOption::builder().build(),
            )
            .unwrap();
            builder.write_all(b"original text\n").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder = EntryBuilder::new_hard_link(
                EntryName::from_lossy("linked1.txt"),
                EntryReference::try_from("origin1.txt").unwrap(),
            )
            .unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder = EntryBuilder::new_hard_link(
                EntryName::from_lossy("dir/linked1.txt"),
                EntryReference::try_from("../origin1.txt").unwrap(),
            )
            .unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer
        .add_entry({
            let mut builder = EntryBuilder::new_file(
                EntryName::from_lossy("dir/origin2.txt"),
                WriteOption::builder().build(),
            )
            .unwrap();
            builder.write_all(b"original text text\n").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder = EntryBuilder::new_hard_link(
                EntryName::from_lossy("dir/linked2.txt"),
                EntryReference::try_from("origin2.txt").unwrap(),
            )
            .unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder = EntryBuilder::new_hard_link(
                EntryName::from_lossy("linked2.txt"),
                EntryReference::try_from("dir/origin2.txt").unwrap(),
            )
            .unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

#[test]
fn hardlink() {
    init_resource("../out/hardlink.pna");
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "../out/hardlink.pna",
        "--overwrite",
        "--out-dir",
        "../out/hardlink/dist",
    ]))
    .unwrap();

    assert_eq!(
        "original text\n",
        fs::read_to_string("../out/hardlink/dist/linked1.txt").unwrap()
    );
    assert_eq!(
        "original text\n",
        fs::read_to_string("../out/hardlink/dist/dir/linked1.txt").unwrap()
    );

    assert_eq!(
        "original text text\n",
        fs::read_to_string("../out/hardlink/dist/dir/linked2.txt").unwrap()
    );
    assert_eq!(
        "original text text\n",
        fs::read_to_string("../out/hardlink/dist/linked2.txt").unwrap()
    );
}
