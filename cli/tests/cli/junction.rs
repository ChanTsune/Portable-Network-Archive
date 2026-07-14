//! Integration tests for Windows junction support.

use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryBuilder, EntryName, EntryReference, LinkTargetType, PermissionMode};
use portable_network_archive::cli;
use std::fs;

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

/// Precondition: a junction passed as its own top-level operand, alongside
/// its target as a separate operand.
/// Action: `pna create` over both operands.
/// Expectation: the junction is encoded as HardLink + fLTP=Directory with the
/// absolute target path stored verbatim as entry data — a top-level junction
/// operand is never relativized because its target is outside its own walk —
/// and the archive contains exactly the direct tree; the junction's contents
/// are not followed and duplicated.
#[test]
#[cfg(windows)]
fn create_records_junction_as_hardlink_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("target_dir");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("inside.txt"), b"hi").unwrap();
    let junction = tmp.path().join("link_dir");
    mklink_junction(&junction, &target);

    let archive_path = tmp.path().join("out.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
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
            assert_eq!(entry.header().data_kind(), DataKind::HardLink);
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

/// Precondition: a directory tree containing a junction that points at its
/// own ancestor.
/// Action: `pna create` over the tree.
/// Expectation: traversal does not recurse through the junction; the archive
/// contains exactly the direct tree plus the junction entry itself, whose
/// stored target is the relative form of the ancestor (`.` for the link's
/// own parent).
#[test]
#[cfg(windows)]
fn create_with_cyclic_junction_does_not_recurse() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file.txt"), b"data").unwrap();
    let junction = root.join("loop");
    mklink_junction(&junction, &root);

    let archive_path = tmp.path().join("cyclic.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
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

/// Reads an entry's data stream as a UTF-8 string (junction entries store
/// their target path as the entry data).
#[cfg(windows)]
fn read_entry_data_string(entry: &pna::NormalEntry<std::borrow::Cow<'_, [u8]>>) -> String {
    let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
    let mut s = String::new();
    std::io::Read::read_to_string(&mut reader, &mut s).unwrap();
    s
}

/// Precondition: a directory tree where a junction and its target directory
/// are siblings under the created root.
/// Action: `pna create` over the root.
/// Expectation: the stored junction target is the target's name relative to
/// the link's parent, not the machine-specific absolute path.
#[test]
#[cfg(windows)]
fn create_stores_in_tree_junction_target_relative() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let target = root.join("target_dir");
    std::fs::create_dir(&target).unwrap();
    let junction = root.join("link_dir");
    mklink_junction(&junction, &target);

    let archive_path = tmp.path().join("rel.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
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

/// Precondition: a directory tree where a junction sits in a subdirectory and
/// points at a directory one level up in the same tree.
/// Action: `pna create` over the root.
/// Expectation: the stored junction target ascends with `..` segments,
/// `/`-separated.
#[test]
#[cfg(windows)]
fn create_stores_nested_junction_target_relative() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    std::fs::create_dir_all(root.join("a")).unwrap();
    let target = root.join("target");
    std::fs::create_dir(&target).unwrap();
    let junction = root.join("a").join("link");
    mklink_junction(&junction, &target);

    let archive_path = tmp.path().join("nested.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
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

/// Precondition: a directory tree containing a junction whose target lies
/// outside the created root.
/// Action: `pna create` over the root only.
/// Expectation: the stored junction target keeps the absolute on-disk form —
/// a relative form could never resolve inside the extracted tree.
#[test]
#[cfg(windows)]
fn create_keeps_out_of_tree_junction_target_absolute() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let external = tmp.path().join("external");
    std::fs::create_dir(&external).unwrap();
    let junction = root.join("link_dir");
    mklink_junction(&junction, &external);

    let archive_path = tmp.path().join("ext.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
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

/// Precondition: an archive created from a tree whose junction points at a
/// sibling directory inside the tree; the source tree is then deleted.
/// Action: extract with `--allow-unsafe-links` into a different directory.
/// Expectation: the junction resolves inside the extracted tree — reading the
/// target's payload through the link succeeds even though the original
/// location no longer exists.
#[test]
#[cfg(windows)]
fn round_trip_junction_into_different_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let target = root.join("target_dir");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("payload.txt"), b"payload").unwrap();
    let junction = root.join("link_dir");
    mklink_junction(&junction, &target);

    let archive_path = tmp.path().join("moved.pna");
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
            .current_dir(tmp.path())
            .args(["create", "-f"])
            .arg(&archive_path)
            .arg("root")
            .status()
            .unwrap()
            .success()
    );

    // Deleting the source tree makes the assertion discriminating: an
    // absolute stored target would still resolve to the original location.
    std::fs::remove_dir_all(&root).unwrap();

    let out_dir = tmp.path().join("elsewhere");
    std::fs::create_dir(&out_dir).unwrap();
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
            .args(["extract", "-f"])
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

/// Build an in-memory archive containing one HardLink+fLTP=Directory entry
/// whose target is the supplied path string (interpreted verbatim).
fn build_junction_fixture(target: &str) -> Vec<u8> {
    build_junction_fixture_with_optional_mode(target, None)
}

/// Like [`build_junction_fixture`] but stamps a permission-mode facet with the
/// given mode. Used by the external-target mutation test: extract with
/// `--keep-permission` would fire mode restoration — which, under a
/// regression that drops the junction arm's early return, would follow the
/// link and mutate the external target's mode.
fn build_junction_fixture_with_mode(target: &str, mode: u16) -> Vec<u8> {
    build_junction_fixture_with_optional_mode(target, Some(mode))
}

fn build_junction_fixture_with_optional_mode(target: &str, mode: Option<u16>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut archive = Archive::write_header(&mut out).unwrap();
    let name = EntryName::from_utf8_preserve_root("link_dir");
    let reference = EntryReference::from_utf8_preserve_root(target);
    let mut builder = EntryBuilder::new_hard_link(name, reference).unwrap();
    builder.link_target_type(LinkTargetType::Directory);
    if let Some(m) = mode {
        builder.permission_mode(PermissionMode::from(m));
    }
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
    out
}

/// Like [`build_junction_fixture`] but stamps a modification timestamp on the
/// junction entry. Used by the `--keep-timestamp` test.
fn build_junction_fixture_with_modified(target: &str, modified: pna::Duration) -> Vec<u8> {
    let mut out = Vec::new();
    let mut archive = Archive::write_header(&mut out).unwrap();
    let name = EntryName::from_utf8_preserve_root("link_dir");
    let reference = EntryReference::from_utf8_preserve_root(target);
    let mut builder = EntryBuilder::new_hard_link(name, reference).unwrap();
    builder.link_target_type(LinkTargetType::Directory);
    builder.modified(modified);
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
    out
}

/// Precondition: archive with a HardLink+fLTP=Directory entry pointing at a
/// well-known absolute path.
/// Action: extract without `--allow-unsafe-links`.
/// Expectation: the entry is skipped and no link is created in the output
/// directory.
#[test]
fn extract_junction_without_allow_unsafe_links_skips() {
    setup();
    let base = "extract_junction_without_allow_unsafe_links_skips";
    let _ = fs::remove_dir_all(base);
    let archive_path = format!("{base}/{base}.pna");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(&archive_path, build_junction_fixture("/any/absolute/path")).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // `symlink_metadata` (not `exists`) so a dangling link created despite the
    // missing flag is still detected: `exists()` follows the link and returns
    // false for a dangling target, hiding exactly the regression this test guards.
    let link = std::path::Path::new(&out_dir).join("link_dir");
    assert!(
        fs::symlink_metadata(&link).is_err(),
        "junction entry must not be extracted without --allow-unsafe-links"
    );
}

/// Precondition: archive with a HardLink+fLTP=Directory entry pointing at an
/// existing absolute path that the test has materialized on disk.
/// Action: extract with `--allow-unsafe-links`.
/// Expectation: on Windows a real junction/reparse-point is created; on
/// non-Windows a symbolic link is created whose target string equals the
/// stored absolute path verbatim.
///
/// Gated off WASM because wasi-preview1 does not expose symlink creation.
#[test]
#[cfg(not(target_family = "wasm"))]
fn extract_junction_with_allow_unsafe_links_creates_link() {
    setup();
    let base = "extract_junction_with_allow_unsafe_links_creates_link";
    let _ = fs::remove_dir_all(base);
    let target_rel = format!("{base}/actual_target");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&target_rel).unwrap();
    fs::create_dir_all(&out_dir).unwrap();
    // The fixture stores the junction target as an absolute path string.
    // `canonicalize` resolves any ancestor symlinks so that on Windows the
    // kernel accepts the path verbatim at `FSCTL_SET_REPARSE_POINT` time.
    let target_abs = fs::canonicalize(&target_rel).unwrap();
    let target_str = target_abs.to_string_lossy().into_owned();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture(&target_str)).unwrap();

    cli::Cli::try_parse_from([
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
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    let meta = fs::symlink_metadata(&link).unwrap();

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

/// Precondition: a directory tree containing a real junction created via
/// `mklink /J`, plus its target materialized on disk.
/// Action: `pna create` over the tree, then `pna extract --allow-unsafe-links`
/// into a fresh output directory.
/// Expectation: the round-tripped link is a junction (not a symbolic link),
/// and reading the target's payload through the link succeeds — proving the
/// reparse point actually resolves to the target directory.
#[test]
#[cfg(windows)]
fn round_trip_junction_via_cli() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("target_dir");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("payload.txt"), b"payload").unwrap();
    let junction = tmp.path().join("link_dir");
    mklink_junction(&junction, &target);

    let archive_path = tmp.path().join("rt.pna");
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
            .current_dir(tmp.path())
            .args(["create", "-f"])
            .arg(&archive_path)
            .args(["link_dir", "target_dir"])
            .status()
            .unwrap()
            .success()
    );

    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    assert!(
        std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
            .args(["extract", "-f"])
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

    // Deep-verify the reparse tag. `Get-Item` exposes the invariant property
    // value `Junction` regardless of system locale (`dir /AL` localizes its
    // `<JUNCTION>` tag). The link path is passed via environment variable to
    // avoid quoting issues.
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

    // Reading through the junction proves the reparse point resolves to the
    // right directory (a broken substitute name would fail here).
    let read_through = std::fs::read(link.join("payload.txt")).unwrap();
    assert_eq!(read_through, b"payload");
}

/// Precondition: archive with a HardLink+fLTP=Directory entry whose target is
/// an external directory the test has pre-populated with a recognizable mode.
/// Action: extract with `--allow-unsafe-links --keep-permission`.
/// Expectation: the junction (Windows) or fallback symlink (non-Windows) is
/// created in the extraction root, AND the external target directory's
/// permissions are byte-for-byte unchanged from the pre-extract state — the
/// junction arm must never apply follow-link metadata syscalls. If a
/// regression re-introduces the default restore_metadata() call for junction
/// entries, this assertion fires.
///
/// Gated off WASM because wasi-preview1 does not expose symlink creation
/// (the fallback path this test exercises).
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

    // Pre-set a recognizable mode on the external target so a follow-link
    // chmod regression would change the observed permissions.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_rel).unwrap().permissions();
        perms.set_mode(0o700);
        fs::set_permissions(&target_rel, perms).unwrap();
    }
    // Canonicalize so the stored target matches what the kernel sees
    // (Windows FSCTL + non-Windows ancestor-symlink handling).
    let target_abs = fs::canonicalize(&target_rel).unwrap();
    let target_str = target_abs.to_string_lossy().into_owned();
    let baseline_perms = fs::metadata(&target_abs).unwrap().permissions();

    // Stamp a permission mode DIFFERENT from the external target's pre-set
    // mode (0o700). Without it this test is decorative: the chmod branch is
    // skipped when no mode facet is present, so a regression that reinstates
    // the default restore_metadata() call for junction entries would pass.
    // With 0o755 stamped, such a regression chmods through the link and
    // changes the external target's mode, firing the assertion below.
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
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // The link must exist.
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

    // The external target's permissions must be byte-for-byte unchanged.
    let after_perms = fs::metadata(&target_abs).unwrap().permissions();
    assert_eq!(
        baseline_perms, after_perms,
        "extract --keep-permission must not mutate the junction's external target"
    );
}

/// Precondition: the output path for a junction entry is already occupied by
/// a directory.
/// Action: extract with `--allow-unsafe-links` but without `--overwrite`.
/// Expectation: extraction fails and the pre-existing directory is left
/// untouched.
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

/// Precondition: the output path for a junction entry is already occupied by
/// a directory.
/// Action: extract with `--allow-unsafe-links --overwrite`.
/// Expectation: the directory is replaced by the link.
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

/// Precondition: archive with a junction entry; the bsdtar-compat surface
/// defaults to allowing unsafe links.
/// Action: `pna compat bsdtar -x` without any link-safety flag.
/// Expectation: the junction entry is extracted.
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

/// Precondition: archive with a junction entry.
/// Action: `pna compat bsdtar -x --no-allow-unsafe-links`.
/// Expectation: the junction entry is skipped.
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

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "compat",
        "bsdtar",
        "-x",
        "--no-allow-unsafe-links",
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
        fs::symlink_metadata(&link).is_err(),
        "junction entry must not be extracted with --no-allow-unsafe-links"
    );
}

/// Precondition: archive with a junction entry whose stored target is a
/// relative path.
/// Action: extract with `--allow-unsafe-links` on a non-Windows host.
/// Expectation: the fallback symlink's target equals the stored relative
/// string verbatim — the non-Windows arm performs no join or resolution.
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

/// Precondition: archive with a real directory (containing a file) and a
/// junction entry whose stored target is a relative path resolvable against
/// the link's parent after extraction.
/// Action: extract with `--allow-unsafe-links`.
/// Expectation: the relative target is resolved lexically against the link's
/// parent; the created junction resolves against the restored tree and the
/// file is readable through it.
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
        .add_entry(EntryBuilder::new_dir("data".into()).build().unwrap())
        .unwrap();
    archive
        .add_entry(EntryBuilder::new_dir("data/real".into()).build().unwrap())
        .unwrap();
    let mut file_builder =
        EntryBuilder::new_file("data/real/inside.txt".into(), pna::WriteOptions::store()).unwrap();
    std::io::Write::write_all(&mut file_builder, b"payload").unwrap();
    archive.add_entry(file_builder.build().unwrap()).unwrap();
    let mut junction_builder = EntryBuilder::new_hard_link(
        EntryName::from_utf8_preserve_root("data/link"),
        EntryReference::from_utf8_preserve_root("real"),
    )
    .unwrap();
    junction_builder.link_target_type(LinkTargetType::Directory);
    archive
        .add_entry(junction_builder.build().unwrap())
        .unwrap();
    archive.finalize().unwrap();

    cli::Cli::try_parse_from([
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

/// Precondition: archive with a junction entry whose stored target is a
/// relative path that exists neither in the archive nor on disk.
/// Action: extract with `--allow-unsafe-links` into a relative output
/// directory.
/// Expectation: the joined target is made absolute lexically without
/// consulting the filesystem, so extraction succeeds and creates a dangling
/// junction.
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

/// Precondition: archive with a junction entry whose stored target is an
/// absolute path.
/// Action: extract with a `-s` substitution rule rewriting the target prefix
/// plus `--allow-unsafe-links`.
/// Expectation: the fallback symlink's target reflects the substitution.
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
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
        "-s",
        "#/before/#/after/#",
        "--unstable",
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

/// Precondition: archive with a junction entry carrying a modification
/// timestamp different from the external target directory's mtime.
/// Action: extract with `--allow-unsafe-links --keep-timestamp`.
/// Expectation: the link's own no-follow mtime equals the archived value and
/// the external target directory's mtime is unchanged.
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
    let mtime = pna::Duration::seconds(1_704_067_200); // 2024-01-01T00:00:00Z
    fs::write(
        &archive_path,
        build_junction_fixture_with_modified(&target_str, mtime),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
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

/// Precondition: a directory tree containing a junction that points at its
/// own ancestor.
/// Action: `pna create --follow-links` over the tree.
/// Expectation: walkdir's loop detection yields an error for the cyclic
/// junction, which create surfaces as a failure — the command terminates
/// instead of hanging.
#[test]
#[cfg(windows)]
fn create_with_follow_links_cyclic_junction_terminates() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file.txt"), b"data").unwrap();
    let junction = root.join("loop");
    mklink_junction(&junction, &root);

    let archive_path = tmp.path().join("cyclic.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
        .arg(&archive_path)
        .args(["--follow-links", "root"])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "following a cyclic junction must terminate with a loop error, not hang"
    );
}

/// Precondition: archive with a junction entry whose stored target is the
/// empty string.
/// Action: extract with `--allow-unsafe-links`.
/// Expectation: extraction fails and no filesystem object exists at the link
/// path.
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
