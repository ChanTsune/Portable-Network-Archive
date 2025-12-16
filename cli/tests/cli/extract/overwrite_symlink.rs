use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryBuilder, WriteOptions, fs as pna_fs};
use portable_network_archive::cli;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn init_symlink_archive<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    writer
        .add_entry({
            let builder =
                EntryBuilder::new_symlink("link".into(), "new_target.txt".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file("new_target.txt".into(), WriteOptions::builder().build())
                    .unwrap();
            builder.write_all(b"updated").unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

#[test]
fn overwrite_symlink_does_not_remove_target_directory() {
    setup();

    init_symlink_archive("overwrite_symlink/archive.pna");

    let dist = PathBuf::from("overwrite_symlink/dist");
    fs::create_dir_all(&dist).unwrap();

    let outside = PathBuf::from("overwrite_symlink/outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("marker.txt"), "keep me").unwrap();

    // Create a pre-existing symlink that points outside the extraction root.
    let link_path = dist.join("link");
    if link_path.exists() {
        pna_fs::remove_path_all(&link_path).unwrap();
    }
    pna_fs::symlink(&outside, &link_path).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "overwrite_symlink/archive.pna",
        "--out-dir",
        "overwrite_symlink/dist",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Ensure the outside directory was not deleted recursively.
    assert!(outside.join("marker.txt").exists());

    // Verify the extracted symlink now points to the expected target.
    assert!(
        fs::symlink_metadata(dist.join("link"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_to_string(dist.join("new_target.txt")).unwrap(),
        "updated"
    );
}
