use crate::utils::setup;
use clap::Parser;
use pna::{
    Archive, EntryName, EntryReference, HardLinkEntryBuilder, LinkTargetType, Metadata,
    PermissionMode,
};
#[cfg(windows)]
use pna::{DirEntryBuilder, FileEntryBuilder};
use portable_network_archive::cli;
use std::fs;
#[cfg(windows)]
use std::path::PathBuf;

#[cfg(windows)]
use pna::{DataKind, ReadEntry, ReadOptions};
#[cfg(windows)]
use std::collections::HashSet;

#[cfg(windows)]
fn mklink_junction(link: &std::path::Path, target: &std::path::Path) {
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .expect("mklink");
    assert!(status.success(), "mklink /J failed");
}

#[cfg(windows)]
fn setup_junction_test_dir(name: &str) -> PathBuf {
    setup();
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if let Err(error) = fs::remove_dir_all(&path) {
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::NotFound,
            "clean junction test directory: {error}"
        );
    }
    fs::create_dir_all(&path).expect("create junction test directory");
    path
}

#[test]
#[cfg(windows)]
fn create_records_junction_as_hardlink_directory() {
    let test_dir = setup_junction_test_dir("create_records_junction_as_hardlink_directory");
    let target = test_dir.as_path().join("target_dir");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("inside.txt"), b"hi").unwrap();
    let junction = test_dir.as_path().join("link_dir");
    mklink_junction(&junction, &target);

    let archive_path = test_dir.as_path().join("out.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(test_dir.as_path())
        .args(["create", "--unstable", "--enable-junction", "-f"])
        .arg(&archive_path)
        .args(["link_dir", "target_dir"])
        .status()
        .unwrap();
    assert!(status.success());

    let bytes = std::fs::read(&archive_path).unwrap();
    let mut archive = Archive::read_header(&bytes[..]).unwrap();
    let mut seen = HashSet::new();
    let mut saw_junction_entry = false;
    for entry in archive.entries_slice() {
        let entry = entry.unwrap();
        let ReadEntry::Normal(entry) = entry else {
            continue;
        };
        seen.insert(entry.header().path().as_str().to_string());
        if entry.header().path().as_str() == "link_dir" {
            assert_eq!(entry.header().data_kind(), DataKind::HARD_LINK);
            assert_eq!(
                entry.metadata().link_target_type(),
                Some(LinkTargetType::Directory)
            );
            let s = read_entry_data_string(&entry);
            let expected = target.to_string_lossy();
            assert_eq!(s, expected, "expected exact absolute target; got {s:?}");
            saw_junction_entry = true;
        }
    }
    assert!(saw_junction_entry, "no HardLink entry found for junction");
    let expected: HashSet<String> = ["link_dir", "target_dir", "target_dir/inside.txt"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        seen, expected,
        "junction contents must not be traversed into the archive"
    );
}

#[test]
#[cfg(windows)]
fn create_with_cyclic_junction_does_not_recurse() {
    let test_dir = setup_junction_test_dir("create_with_cyclic_junction_does_not_recurse");
    let root = test_dir.as_path().join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file.txt"), b"data").unwrap();
    let junction = root.join("loop");
    mklink_junction(&junction, &root);

    let archive_path = test_dir.as_path().join("cyclic.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(test_dir.as_path())
        .args(["create", "--unstable", "--enable-junction", "-f"])
        .arg(&archive_path)
        .arg("root")
        .status()
        .unwrap();
    assert!(status.success());

    let bytes = std::fs::read(&archive_path).unwrap();
    let mut archive = Archive::read_header(&bytes[..]).unwrap();
    let mut seen = HashSet::new();
    for entry in archive.entries_slice() {
        let entry = entry.unwrap();
        let ReadEntry::Normal(entry) = entry else {
            continue;
        };
        seen.insert(entry.header().path().as_str().to_string());
        if entry.header().path().as_str() == "root/loop" {
            assert_eq!(read_entry_data_string(&entry), ".");
        }
    }
    let expected: HashSet<String> = ["root", "root/file.txt", "root/loop"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        seen, expected,
        "cyclic junction must not be recursed into during create"
    );
}

#[cfg(windows)]
fn read_entry_data_string(entry: &pna::NormalEntry<std::borrow::Cow<'_, [u8]>>) -> String {
    let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut reader, &mut s).unwrap();
    s
}

#[test]
#[cfg(windows)]
fn create_stores_in_tree_junction_target_relative() {
    let test_dir = setup_junction_test_dir("create_stores_in_tree_junction_target_relative");
    let root = test_dir.as_path().join("root");
    std::fs::create_dir(&root).unwrap();
    let target = root.join("target_dir");
    std::fs::create_dir(&target).unwrap();
    let junction = root.join("link_dir");
    mklink_junction(&junction, &target);

    let archive_path = test_dir.as_path().join("rel.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(test_dir.as_path())
        .args(["create", "--unstable", "--enable-junction", "-f"])
        .arg(&archive_path)
        .arg("root")
        .status()
        .unwrap();
    assert!(status.success());

    let bytes = std::fs::read(&archive_path).unwrap();
    let mut archive = Archive::read_header(&bytes[..]).unwrap();
    let mut saw_junction_entry = false;
    for entry in archive.entries_slice() {
        let entry = entry.unwrap();
        let ReadEntry::Normal(entry) = entry else {
            continue;
        };
        if entry.header().path().as_str() == "root/link_dir" {
            assert_eq!(read_entry_data_string(&entry), "target_dir");
            saw_junction_entry = true;
        }
    }
    assert!(saw_junction_entry, "no entry found for junction");
}

#[test]
#[cfg(windows)]
fn create_stores_nested_junction_target_relative() {
    let test_dir = setup_junction_test_dir("create_stores_nested_junction_target_relative");
    let root = test_dir.as_path().join("root");
    std::fs::create_dir_all(root.join("a")).unwrap();
    let target = root.join("target");
    std::fs::create_dir(&target).unwrap();
    let junction = root.join("a").join("link");
    mklink_junction(&junction, &target);

    let archive_path = test_dir.as_path().join("nested.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(test_dir.as_path())
        .args(["create", "--unstable", "--enable-junction", "-f"])
        .arg(&archive_path)
        .arg("root")
        .status()
        .unwrap();
    assert!(status.success());

    let bytes = std::fs::read(&archive_path).unwrap();
    let mut archive = Archive::read_header(&bytes[..]).unwrap();
    let mut saw_junction_entry = false;
    for entry in archive.entries_slice() {
        let entry = entry.unwrap();
        let ReadEntry::Normal(entry) = entry else {
            continue;
        };
        if entry.header().path().as_str() == "root/a/link" {
            assert_eq!(read_entry_data_string(&entry), "../target");
            saw_junction_entry = true;
        }
    }
    assert!(saw_junction_entry, "no entry found for junction");
}

#[test]
#[cfg(windows)]
fn create_keeps_out_of_tree_junction_target_absolute() {
    let test_dir = setup_junction_test_dir("create_keeps_out_of_tree_junction_target_absolute");
    let root = test_dir.as_path().join("root");
    std::fs::create_dir(&root).unwrap();
    let external = test_dir.as_path().join("external");
    std::fs::create_dir(&external).unwrap();
    let junction = root.join("link_dir");
    mklink_junction(&junction, &external);

    let archive_path = test_dir.as_path().join("ext.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(test_dir.as_path())
        .args(["create", "--unstable", "--enable-junction", "-f"])
        .arg(&archive_path)
        .arg("root")
        .status()
        .unwrap();
    assert!(status.success());

    let bytes = std::fs::read(&archive_path).unwrap();
    let mut archive = Archive::read_header(&bytes[..]).unwrap();
    let mut saw_junction_entry = false;
    for entry in archive.entries_slice() {
        let entry = entry.unwrap();
        let ReadEntry::Normal(entry) = entry else {
            continue;
        };
        if entry.header().path().as_str() == "root/link_dir" {
            assert_eq!(read_entry_data_string(&entry), external.to_string_lossy());
            saw_junction_entry = true;
        }
    }
    assert!(saw_junction_entry, "no entry found for junction");
}

#[test]
#[cfg(windows)]
fn round_trip_junction_into_different_directory() {
    let test_dir = setup_junction_test_dir("round_trip_junction_into_different_directory");
    let root = test_dir.as_path().join("root");
    std::fs::create_dir(&root).unwrap();
    let target = root.join("target_dir");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("payload.txt"), b"payload").unwrap();
    let junction = root.join("link_dir");
    mklink_junction(&junction, &target);

    let archive_path = test_dir.as_path().join("moved.pna");
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
            .current_dir(test_dir.as_path())
            .args(["create", "--unstable", "--enable-junction", "-f"])
            .arg(&archive_path)
            .arg("root")
            .status()
            .unwrap()
            .success()
    );

    std::fs::remove_dir_all(&root).unwrap();

    let out_dir = test_dir.as_path().join("elsewhere");
    std::fs::create_dir(&out_dir).unwrap();
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
            .args(["extract", "--unstable", "--enable-junction", "-f"])
            .arg(&archive_path)
            .arg("--out-dir")
            .arg(&out_dir)
            .arg("--allow-unsafe-links")
            .status()
            .unwrap()
            .success()
    );

    let link = out_dir.join("root").join("link_dir");
    let read_through = std::fs::read(link.join("payload.txt")).unwrap();
    assert_eq!(
        read_through, b"payload",
        "the junction must resolve inside the extracted tree, not the original location"
    );
}

fn build_junction_fixture(target: &str) -> Vec<u8> {
    build_junction_fixture_with_optional_mode(target, None)
}

/// Add a mode facet to detect metadata writes through the junction.
fn build_junction_fixture_with_mode(target: &str, mode: u16) -> Vec<u8> {
    build_junction_fixture_with_optional_mode(target, Some(mode))
}

fn build_junction_fixture_with_optional_mode(target: &str, mode: Option<u16>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut archive = Archive::write_header(&mut out).unwrap();
    let name = EntryName::from_utf8_preserve_root("link_dir");
    let reference = EntryReference::from_utf8_preserve_root(target);
    let mut metadata = Metadata::new().with_link_target_type(Some(LinkTargetType::Directory));
    if let Some(m) = mode {
        metadata = metadata.with_permission_mode(Some(PermissionMode::from(m)));
    }
    let mut builder = HardLinkEntryBuilder::new(name, reference).unwrap();
    builder.metadata(metadata);
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
    out
}

fn build_junction_fixture_with_modified(target: &str, modified: pna::Duration) -> Vec<u8> {
    let mut out = Vec::new();
    let mut archive = Archive::write_header(&mut out).unwrap();
    let name = EntryName::from_utf8_preserve_root("link_dir");
    let reference = EntryReference::from_utf8_preserve_root(target);
    let mut builder = HardLinkEntryBuilder::new(name, reference).unwrap();
    builder.metadata(
        Metadata::new()
            .with_link_target_type(Some(LinkTargetType::Directory))
            .with_modified(Some(modified)),
    );
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
    out
}

#[test]
fn extract_junction_without_enable_junction_skips() {
    setup();
    let base = "extract_junction_without_enable_junction_skips";
    let _ = fs::remove_dir_all(base);
    let archive_path = format!("{base}/{base}.pna");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(&archive_path, build_junction_fixture("/any/absolute/path")).unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "skipping a junction must report a security-skip failure"
    );
    let link = std::path::Path::new(&out_dir).join("link_dir");
    assert!(
        fs::symlink_metadata(&link).is_err(),
        "junction entry must not be extracted without --enable-junction"
    );
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn extract_junction_with_allow_unsafe_links_creates_link() {
    setup();
    let base = "extract_junction_with_allow_unsafe_links_creates_link";
    let _ = fs::remove_dir_all(base);
    let target_rel = format!("{base}/actual_target");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&target_rel).unwrap();
    fs::write(format!("{target_rel}/payload.txt"), b"payload").unwrap();
    fs::create_dir_all(&out_dir).unwrap();
    let target_abs = fs::canonicalize(&target_rel).unwrap();
    let target_str = target_abs.to_string_lossy().into_owned();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture(&target_str)).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    let meta = fs::symlink_metadata(&link).unwrap();
    assert_eq!(
        fs::read(link.join("payload.txt")).unwrap(),
        b"payload",
        "the link must resolve to the canonicalized target directory"
    );

    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        let ft = meta.file_type();
        assert!(
            ft.is_symlink() || ft.is_symlink_dir() || ft.is_symlink_file(),
            "expected a reparse-point flavored link at {}; got {:?}",
            link.display(),
            ft
        );
    }
    #[cfg(not(windows))]
    {
        assert!(meta.file_type().is_symlink());
        assert_eq!(
            fs::read_link(&link).unwrap(),
            std::path::PathBuf::from(&target_str)
        );
    }
}

#[test]
#[cfg(windows)]
fn round_trip_junction_via_cli() {
    let test_dir = setup_junction_test_dir("round_trip_junction_via_cli");
    let target = test_dir.as_path().join("target_dir");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("payload.txt"), b"payload").unwrap();
    let junction = test_dir.as_path().join("link_dir");
    mklink_junction(&junction, &target);

    let archive_path = test_dir.as_path().join("rt.pna");
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
            .current_dir(test_dir.as_path())
            .args(["create", "--unstable", "--enable-junction", "-f"])
            .arg(&archive_path)
            .args(["link_dir", "target_dir"])
            .status()
            .unwrap()
            .success()
    );

    let out_dir = test_dir.as_path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
            .args(["extract", "--unstable", "--enable-junction", "-f"])
            .arg(&archive_path)
            .arg("--out-dir")
            .arg(&out_dir)
            .arg("--allow-unsafe-links")
            .status()
            .unwrap()
            .success()
    );

    let link = out_dir.join("link_dir");
    let meta = std::fs::symlink_metadata(&link).unwrap();
    use std::os::windows::fs::FileTypeExt;
    let ft = meta.file_type();
    assert!(
        ft.is_symlink() || ft.is_symlink_dir(),
        "expected a reparse point, got {ft:?}"
    );

    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-Item -LiteralPath $env:PNA_TEST_LINK -Force).LinkType",
        ])
        .env("PNA_TEST_LINK", &link)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "Junction",
        "expected LinkType Junction for {}",
        link.display()
    );

    let read_through = std::fs::read(link.join("payload.txt")).unwrap();
    assert_eq!(
        read_through, b"payload",
        "the junction must resolve to the target directory"
    );
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn extract_junction_does_not_mutate_external_target() {
    setup();
    let base = "extract_junction_does_not_mutate_external_target";
    let _ = fs::remove_dir_all(base);
    let target_rel = format!("{base}/external_target");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&target_rel).unwrap();
    fs::create_dir_all(&out_dir).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_rel).unwrap().permissions();
        perms.set_mode(0o700);
        fs::set_permissions(&target_rel, perms).unwrap();
    }
    let target_abs = fs::canonicalize(&target_rel).unwrap();
    let target_str = target_abs.to_string_lossy().into_owned();
    let baseline_perms = fs::metadata(&target_abs).unwrap().permissions();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(
        &archive_path,
        build_junction_fixture_with_mode(&target_str, 0o755),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    let link_meta = fs::symlink_metadata(&link).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        let ft = link_meta.file_type();
        assert!(
            ft.is_symlink() || ft.is_symlink_dir() || ft.is_symlink_file(),
            "expected a reparse-point flavored link at {}; got {ft:?}",
            link.display()
        );
    }
    #[cfg(not(windows))]
    {
        assert!(
            link_meta.file_type().is_symlink(),
            "expected a symlink at {}; got {:?}",
            link.display(),
            link_meta.file_type()
        );
    }

    let after_perms = fs::metadata(&target_abs).unwrap().permissions();
    assert_eq!(
        baseline_perms, after_perms,
        "extract --keep-permission must not mutate the junction's external target"
    );
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn extract_junction_over_existing_path_without_overwrite_fails() {
    setup();
    let base = "extract_junction_over_existing_path_without_overwrite_fails";
    let _ = fs::remove_dir_all(base);
    let target_rel = format!("{base}/actual_target");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&target_rel).unwrap();
    fs::create_dir_all(format!("{out_dir}/link_dir")).unwrap();
    let target_str = fs::canonicalize(&target_rel)
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture(&target_str)).unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "extracting a junction over an existing path without --overwrite must fail"
    );
    let link = std::path::Path::new(&out_dir).join("link_dir");
    assert!(
        fs::symlink_metadata(&link).unwrap().file_type().is_dir(),
        "the pre-existing directory must be left in place"
    );
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn extract_junction_over_existing_path_with_overwrite_replaces() {
    setup();
    let base = "extract_junction_over_existing_path_with_overwrite_replaces";
    let _ = fs::remove_dir_all(base);
    let target_rel = format!("{base}/actual_target");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&target_rel).unwrap();
    fs::create_dir_all(format!("{out_dir}/link_dir")).unwrap();
    let target_str = fs::canonicalize(&target_rel)
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture(&target_str)).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    let meta = fs::symlink_metadata(&link).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        let ft = meta.file_type();
        assert!(
            ft.is_symlink() || ft.is_symlink_dir(),
            "expected the directory to be replaced by a reparse point; got {ft:?}"
        );
    }
    #[cfg(not(windows))]
    {
        assert!(
            meta.file_type().is_symlink(),
            "expected the directory to be replaced by a symlink; got {:?}",
            meta.file_type()
        );
    }
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn bsdtar_extract_junction_by_default() {
    setup();
    let base = "bsdtar_extract_junction_by_default";
    let _ = fs::remove_dir_all(base);
    let target_rel = format!("{base}/actual_target");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&target_rel).unwrap();
    fs::create_dir_all(&out_dir).unwrap();
    let target_str = fs::canonicalize(&target_rel)
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture(&target_str)).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "compat",
        "bsdtar",
        "--unstable",
        "--enable-junction",
        "-x",
        "--file",
        &archive_path,
        "--out-dir",
        &out_dir,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    assert!(
        fs::symlink_metadata(&link).is_ok(),
        "bsdtar-compat extraction allows unsafe links by default; the junction entry must be extracted"
    );
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn bsdtar_extract_junction_with_no_allow_unsafe_links_skips() {
    setup();
    let base = "bsdtar_extract_junction_with_no_allow_unsafe_links_skips";
    let _ = fs::remove_dir_all(base);
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&out_dir).unwrap();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture("/any/absolute/path")).unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "compat",
        "bsdtar",
        "--unstable",
        "--enable-junction",
        "-x",
        "--no-allow-unsafe-links",
        "--file",
        &archive_path,
        "--out-dir",
        &out_dir,
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "skipping a junction must report a security-skip failure"
    );
    let link = std::path::Path::new(&out_dir).join("link_dir");
    assert!(
        fs::symlink_metadata(&link).is_err(),
        "junction entry must not be extracted with --no-allow-unsafe-links"
    );
}

#[test]
#[cfg(not(any(windows, target_family = "wasm")))]
fn extract_junction_with_relative_target_passes_through_verbatim() {
    setup();
    let base = "extract_junction_with_relative_target_passes_through_verbatim";
    let _ = fs::remove_dir_all(base);
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&out_dir).unwrap();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture("real")).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    assert_eq!(
        fs::read_link(&link).unwrap(),
        std::path::PathBuf::from("real"),
        "relative junction target must pass through to the symlink unchanged"
    );
}

#[test]
#[cfg(windows)]
fn extract_junction_with_relative_target_resolves() {
    setup();
    let base = "extract_junction_with_relative_target_resolves";
    let _ = fs::remove_dir_all(base);
    let out_dir = format!("{base}/out");
    fs::create_dir_all(base).unwrap();

    let archive_path = format!("{base}/{base}.pna");
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    archive
        .add_entry(DirEntryBuilder::new("data".into()).build().unwrap())
        .unwrap();
    archive
        .add_entry(DirEntryBuilder::new("data/real".into()).build().unwrap())
        .unwrap();
    let mut file_builder = FileEntryBuilder::new_with_options(
        "data/real/inside.txt".into(),
        pna::WriteOptions::store(),
    )
    .unwrap();
    std::io::Write::write_all(&mut file_builder, b"payload").unwrap();
    archive.add_entry(file_builder.build().unwrap()).unwrap();
    let mut junction_builder = HardLinkEntryBuilder::new(
        EntryName::from_utf8_preserve_root("data/link"),
        EntryReference::from_utf8_preserve_root("real"),
    )
    .unwrap();
    junction_builder
        .metadata(Metadata::new().with_link_target_type(Some(LinkTargetType::Directory)));
    archive
        .add_entry(junction_builder.build().unwrap())
        .unwrap();
    archive.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("data/link");
    let meta = fs::symlink_metadata(&link).unwrap();
    use std::os::windows::fs::FileTypeExt;
    let ft = meta.file_type();
    assert!(
        ft.is_symlink() || ft.is_symlink_dir(),
        "expected a reparse point at {}; got {ft:?}",
        link.display()
    );
    let read_through = fs::read(link.join("inside.txt")).unwrap();
    assert_eq!(
        read_through, b"payload",
        "the junction must resolve to the sibling directory restored earlier"
    );
}

#[test]
#[cfg(windows)]
fn extract_junction_with_unresolvable_relative_target_creates_dangling() {
    setup();
    let base = "extract_junction_with_unresolvable_relative_target_creates_dangling";
    let _ = fs::remove_dir_all(base);
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&out_dir).unwrap();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture("missing_target")).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    let meta = fs::symlink_metadata(&link).unwrap();
    use std::os::windows::fs::FileTypeExt;
    let ft = meta.file_type();
    assert!(
        ft.is_symlink() || ft.is_symlink_dir(),
        "expected a dangling reparse point at {}; got {ft:?}",
        link.display()
    );
    assert!(
        fs::metadata(&link).is_err(),
        "the junction target must be absent — following the link must fail"
    );
}

#[test]
#[cfg(not(any(windows, target_family = "wasm")))]
fn extract_junction_with_substitution_rewrites_target() {
    setup();
    let base = "extract_junction_with_substitution_rewrites_target";
    let _ = fs::remove_dir_all(base);
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&out_dir).unwrap();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture("/before/target")).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
        "-s",
        "#/before/#/after/#",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    assert_eq!(
        fs::read_link(&link).unwrap(),
        std::path::PathBuf::from("/after/target"),
        "the substitution rule must apply to the junction target"
    );
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn extract_junction_with_keep_timestamp_restores_link_times() {
    setup();
    let base = "extract_junction_with_keep_timestamp_restores_link_times";
    let _ = fs::remove_dir_all(base);
    let target_rel = format!("{base}/external_target");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&target_rel).unwrap();
    fs::create_dir_all(&out_dir).unwrap();
    let target_abs = fs::canonicalize(&target_rel).unwrap();
    let target_str = target_abs.to_string_lossy().into_owned();
    let baseline_mtime = fs::metadata(&target_abs).unwrap().modified().unwrap();

    let archive_path = format!("{base}/{base}.pna");
    let mtime = pna::Duration::seconds(1_704_067_200);
    fs::write(
        &archive_path,
        build_junction_fixture_with_modified(&target_str, mtime),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    let link_mtime = fs::symlink_metadata(&link).unwrap().modified().unwrap();
    let link_mtime_secs = link_mtime
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(
        link_mtime_secs, 1_704_067_200,
        "the junction's own no-follow mtime must equal the archived value"
    );

    let after_mtime = fs::metadata(&target_abs).unwrap().modified().unwrap();
    assert_eq!(
        baseline_mtime, after_mtime,
        "--keep-timestamp must not mutate the junction's external target"
    );
}

#[test]
#[cfg(windows)]
fn create_with_follow_links_cyclic_junction_terminates() {
    let test_dir = setup_junction_test_dir("create_with_follow_links_cyclic_junction_terminates");
    let root = test_dir.as_path().join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file.txt"), b"data").unwrap();
    let junction = root.join("loop");
    mklink_junction(&junction, &root);

    let archive_path = test_dir.as_path().join("cyclic.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(test_dir.as_path())
        .args(["create", "--unstable", "--enable-junction", "-f"])
        .arg(&archive_path)
        .args(["--follow-links", "root"])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "following a cyclic junction must terminate with a loop error, not hang"
    );
}

#[test]
fn extract_junction_with_empty_target_fails() {
    setup();
    let base = "extract_junction_with_empty_target_fails";
    let _ = fs::remove_dir_all(base);
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&out_dir).unwrap();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture("")).unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--unstable",
        "--enable-junction",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "an empty junction target must fail extraction"
    );
    let link = std::path::Path::new(&out_dir).join("link_dir");
    assert!(
        fs::symlink_metadata(&link).is_err(),
        "no filesystem object must be created at the link path"
    );
}
