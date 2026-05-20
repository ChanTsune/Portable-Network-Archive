use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryBuilder};
use portable_network_archive::cli;
use std::{fs, path::Path};

fn init_huge_link_resource<P: AsRef<Path>>(path: P, is_hardlink: bool) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    // Create a target string > 64 KiB
    let huge_target = "a".repeat(64 * 1024 + 1);
    writer
        .add_entry({
            let builder = if is_hardlink {
                EntryBuilder::new_hard_link("huge_link".into(), huge_target.into()).unwrap()
            } else {
                EntryBuilder::new_symlink("huge_link".into(), huge_target.into()).unwrap()
            };
            builder.build().unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

#[test]
fn extract_huge_symlink_fails() {
    setup();
    let pna_file = "extract_huge_symlink/huge.pna";
    init_huge_link_resource(pna_file, false);
    let result = cli::Cli::try_parse_from([
        "pna",
        "x",
        pna_file,
        "--out-dir",
        "extract_huge_symlink/dist",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    let err = format!("{:?}", result.unwrap_err());
    eprintln!("Error: {}", err);
    assert!(err.contains("Symbolic link target is too long"));
}

#[test]
fn extract_huge_hardlink_fails() {
    setup();
    let pna_file = "extract_huge_hardlink/huge.pna";
    init_huge_link_resource(pna_file, true);
    let result = cli::Cli::try_parse_from([
        "pna",
        "x",
        pna_file,
        "--out-dir",
        "extract_huge_hardlink/dist",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    let err = format!("{:?}", result.unwrap_err());
    eprintln!("Error: {}", err);
    assert!(err.contains("Hard link target is too long"));
}
