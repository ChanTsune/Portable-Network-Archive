use clap::Parser;
use pna::{Archive, EntryBuilder, WriteOption};
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
            let mut builder =
                EntryBuilder::new_file("origin1.txt".into(), WriteOption::builder().build())
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
                EntryBuilder::new_file("dir/origin2.txt".into(), WriteOption::builder().build())
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
    init_resource(format!("{}/hardlink.pna", env!("CARGO_TARGET_TMPDIR")));
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/hardlink.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/hardlink/dist", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();

    assert_eq!(
        "original text\n",
        fs::read_to_string(format!(
            "{}/hardlink/dist/linked1.txt",
            env!("CARGO_TARGET_TMPDIR")
        ))
        .unwrap()
    );
    assert_eq!(
        "original text\n",
        fs::read_to_string(format!(
            "{}/hardlink/dist/dir/linked1.txt",
            env!("CARGO_TARGET_TMPDIR")
        ))
        .unwrap()
    );

    assert_eq!(
        "original text text\n",
        fs::read_to_string(format!(
            "{}/hardlink/dist/dir/linked2.txt",
            env!("CARGO_TARGET_TMPDIR")
        ))
        .unwrap()
    );
    assert_eq!(
        "original text text\n",
        fs::read_to_string(format!(
            "{}/hardlink/dist/linked2.txt",
            env!("CARGO_TARGET_TMPDIR")
        ))
        .unwrap()
    );
}
