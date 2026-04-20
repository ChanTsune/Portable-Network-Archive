//! Integration tests for Windows junction support.

use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryBuilder, EntryName, EntryReference, LinkTargetType, Permission};
use portable_network_archive::cli;
use std::fs;

#[cfg(windows)]
use pna::{DataKind, ReadEntry, ReadOptions, prelude::*};

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

/// Precondition: a directory tree containing a junction.
/// Action: `pna create` over the tree.
/// Expectation: the junction is encoded as HardLink + fLTP=Directory with the
/// absolute target path stored verbatim as entry data.
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
    let mut saw_junction_entry = false;
    for entry in archive.entries_slice() {
        let entry = entry.unwrap();
        let ReadEntry::Normal(entry) = entry else {
            continue;
        };
        if entry.header().path().as_str() == "link_dir" {
            assert_eq!(entry.header().data_kind(), DataKind::HardLink);
            assert_eq!(
                entry.metadata().link_target_type(),
                Some(LinkTargetType::Directory)
            );
            let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut reader, &mut s).unwrap();
            let expected = target.to_string_lossy();
            assert_eq!(s, expected, "expected exact absolute target; got {s:?}");
            saw_junction_entry = true;
        }
    }
    assert!(saw_junction_entry, "no HardLink entry found for junction");
}

/// Build an in-memory archive containing one HardLink+fLTP=Directory entry
/// whose target is the supplied path string (interpreted verbatim).
fn build_junction_fixture(target: &str) -> Vec<u8> {
    build_junction_fixture_with_optional_permission(target, None)
}

/// Like [`build_junction_fixture`] but stamps a [`Permission`] chunk with the
/// given mode. Used by the I2 security fence so that extract with
/// `--keep-permission` would fire `restore_mode` — which, under a regression
/// that drops Task 4.1's early return, would follow the link and mutate the
/// external target's mode.
fn build_junction_fixture_with_permission(target: &str, mode: u16) -> Vec<u8> {
    build_junction_fixture_with_optional_permission(
        target,
        Some(Permission::new(0, String::new(), 0, String::new(), mode)),
    )
}

fn build_junction_fixture_with_optional_permission(
    target: &str,
    permission: Option<Permission>,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut archive = Archive::write_header(&mut out).unwrap();
    let name = EntryName::from_utf8_preserve_root("link_dir");
    let reference = EntryReference::from_utf8_preserve_root(target);
    let mut builder = EntryBuilder::new_hard_link(name, reference).unwrap();
    builder.link_target_type(LinkTargetType::Directory);
    if let Some(p) = permission {
        builder.permission(p);
    }
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

    assert!(!std::path::Path::new(&out_dir).join("link_dir").exists());
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
/// Expectation: the round-tripped link is a reparse-point flavored entry
/// (`FileTypeExt::is_symlink()` or `is_symlink_dir()`) AND `dir /AL`
/// identifies it as `JUNCTION`.
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

    // Deep-verify the reparse tag via cmd (avoids needing to expose internal
    // helpers across crate boundaries).
    let output = std::process::Command::new("cmd")
        .args(["/C", "dir", "/AL"])
        .arg(&out_dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("JUNCTION"),
        "expected directory listing to mark link_dir as JUNCTION; got {stdout}"
    );
}

/// Precondition: archive with a HardLink+fLTP=Directory entry whose target is
/// an external directory the test has pre-populated with a recognizable mode.
/// Action: extract with `--allow-unsafe-links --keep-permission`.
/// Expectation: the junction (Windows) or fallback symlink (non-Windows) is
/// created in the extraction root, AND the external target directory's
/// permissions are byte-for-byte unchanged from the pre-extract state. This
/// pins the "junction extract does not apply follow-link metadata syscalls"
/// invariant (I2) from the spec §7. If a regression re-introduces the default
/// restore_metadata() call for junction entries, this assertion fires.
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

    // Stamp a Permission chunk with a mode DIFFERENT from the external
    // target's pre-set mode (0o700). Without this the fence is decorative:
    // extract code's chmod branch is skipped when `metadata().permission()`
    // is None, and a real I2 regression (default restore_metadata call in
    // the junction arm) slips through because no follow-link syscall ever
    // fires. With mode 0o755 stamped, a regression chmod(link, 0o755)
    // would follow the link and change the external target's mode,
    // triggering the `baseline_perms != after_perms` assertion below.
    let archive_path = format!("{base}/{base}.pna");
    fs::write(
        &archive_path,
        build_junction_fixture_with_permission(&target_str, 0o755),
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
        "I2 violation: extract --keep-permission mutated the external junction target's permissions"
    );
}
