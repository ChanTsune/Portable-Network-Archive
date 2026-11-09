use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryName, EntryReference, HardLinkEntryBuilder, SymlinkEntryBuilder};
use portable_network_archive::cli;
use std::{fs, path::Path};

fn build_archive_with_parent_dir_links(archive_path: &Path) {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(archive_path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    writer
        .add_entry({
            SymlinkEntryBuilder::new(
                EntryName::from_utf8_preserve_root("./a/link"),
                EntryReference::from_utf8_preserve_root("../../file.txt"),
            )
            .unwrap()
            .build()
            .unwrap()
        })
        .unwrap();

    writer
        .add_entry({
            HardLinkEntryBuilder::new(
                EntryName::from_utf8_preserve_root("./b/hard"),
                EntryReference::from_utf8_preserve_root("../file.txt"),
            )
            .unwrap()
            .build()
            .unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

/// Precondition: A file exists outside the extraction directory and an archive contains a symlink whose target uses parent directory traversal (..) and a hardlink whose target uses parent directory traversal (..)
/// Action: Extract with --allow-unsafe-links
/// Expectation: Extraction succeeds; the symlink is created with the target preserved verbatim and both links resolve to the file
#[test]
fn extract_allow_unsafe_links_flag_allows_parent_dir_link_targets() {
    setup();

    let root = Path::new("extract_allow_unsafe_links_flag_allows_parent_dir_targets");
    let out_dir = root.join("out");
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("file.txt"), b"content").unwrap();
    build_archive_with_parent_dir_links(&root.join("archive.pna"));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &root.join("archive.pna").to_string_lossy(),
        "--allow-unsafe-links",
        "--overwrite",
        "--out-dir",
        &out_dir.to_string_lossy(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link_path = out_dir.join("a/link");
    let meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(meta.file_type().is_symlink(), "symlink should be created");
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        Path::new("../../file.txt"),
        "symlink target should be preserved verbatim"
    );
    assert_eq!(
        fs::read_to_string(&link_path).unwrap(),
        "content",
        "symlink should resolve to the file"
    );

    let hard_path = out_dir.join("b/hard");
    assert!(hard_path.exists(), "hardlink should be created");
    #[cfg(not(target_family = "wasm"))]
    assert!(
        same_file::is_same_file(&hard_path, root.join("file.txt")).unwrap(),
        "hardlink should link to the file"
    );
}

/// Precondition: A file exists outside the extraction directory and an archive contains a symlink whose target uses parent directory traversal (..) and a hardlink whose target uses parent directory traversal (..)
/// Action: Extract without the flag, then extract with --no-allow-unsafe-links into another directory
/// Expectation: Both extractions fail; the symlink and the hardlink are not created and the file is left untouched
#[test]
fn extract_blocks_parent_dir_link_targets_by_default_and_with_no_allow_flag() {
    setup();

    let root = Path::new("extract_blocks_parent_dir_link_targets_by_default_and_no_allow_flag");
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("file.txt"), b"content").unwrap();
    build_archive_with_parent_dir_links(&root.join("archive.pna"));

    for (out_dir, flag) in [("out", None), ("out2", Some("--no-allow-unsafe-links"))] {
        let mut args = vec![
            "pna".to_string(),
            "--quiet".to_string(),
            "x".to_string(),
            "-f".to_string(),
            root.join("archive.pna").to_string_lossy().into_owned(),
            "--overwrite".to_string(),
            "--out-dir".to_string(),
            root.join(out_dir).to_string_lossy().into_owned(),
        ];
        args.extend(flag.map(str::to_string));
        let result = cli::Cli::try_parse_from(args).unwrap().execute();

        assert!(result.is_err(), "extraction into {out_dir} should fail");
        assert!(
            !fs::exists(root.join(out_dir).join("a/link")).unwrap(),
            "symlink with .. target should be blocked"
        );
        assert!(
            !fs::exists(root.join(out_dir).join("b/hard")).unwrap(),
            "hardlink with .. target should be blocked"
        );
    }

    assert_eq!(
        fs::read(root.join("file.txt")).unwrap(),
        b"content",
        "file outside the extraction directories should be untouched"
    );
}
