use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, path::Path, path::PathBuf};

fn init_symlink_resource(dir: &str) {
    let dir = Path::new(dir);
    if dir.exists() {
        fs::remove_dir_all(dir).unwrap();
    }
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("target.txt"), b"content").unwrap();
    pna::fs::symlink(Path::new("target.txt"), dir.join("link.txt")).unwrap();
}

fn init_broken_symlink_resource(dir: &str) {
    let dir = Path::new(dir);
    if dir.exists() {
        fs::remove_dir_all(dir).unwrap();
    }
    fs::create_dir_all(dir).unwrap();
    pna::fs::symlink(Path::new("nonexistent"), dir.join("broken.txt")).unwrap();
}

/// Precondition: A regular file and a symlink pointing to it exist.
/// Action: Run `pna create` without `--follow-links`, with `--keep-permission`.
/// Expectation: The symlink entry in the archive has its own permission metadata stored.
#[cfg(unix)]
#[test]
fn create_symlink_stores_own_permissions() {
    setup();
    let base = "create_symlink_stores_own_permissions";
    let source = format!("{base}/source");
    init_symlink_resource(&source);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--keep-dir",
        "--keep-permission",
        "--unstable",
        &source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found_symlink = false;
    archive::for_each_entry(format!("{base}/{base}.pna"), |entry| {
        if entry.header().data_kind() == pna::DataKind::SymbolicLink {
            let perm = entry
                .metadata()
                .permission()
                .expect("symlink entry should have permission metadata");
            // On most Unix systems, symlink permissions are 0o777 (0o120777 with type bits).
            // We only verify that permission metadata exists and contains the symlink type bit.
            let mode = perm.permissions();
            assert!(
                mode & 0o777 != 0,
                "symlink permission bits should be non-zero, got {mode:#o}"
            );
            found_symlink = true;
        }
    })
    .unwrap();
    assert!(found_symlink, "no symlink entry found in archive");
}

/// Precondition: A symlink with a known modification timestamp exists.
/// Action: Run `pna create` with `--keep-timestamp`.
/// Expectation: The symlink entry in the archive has its own mtime stored (from symlink_metadata, not target).
#[test]
fn create_symlink_stores_own_timestamps() {
    setup();
    let base = "create_symlink_stores_own_timestamps";
    let source = format!("{base}/source");
    init_symlink_resource(&source);

    // Get the symlink's own mtime before archiving
    let link_meta = fs::symlink_metadata(format!("{source}/link.txt")).unwrap();
    let link_mtime = link_meta.modified().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--keep-dir",
        "--keep-timestamp",
        &source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Convert SystemTime to epoch seconds for comparison
    let link_mtime_secs = link_mtime
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut found_symlink = false;
    archive::for_each_entry(format!("{base}/{base}.pna"), |entry| {
        if entry.header().data_kind() == pna::DataKind::SymbolicLink {
            let archived_mtime = entry
                .metadata()
                .modified()
                .expect("symlink entry should have mtime");
            let archived_secs = archived_mtime.whole_seconds();
            let diff = (archived_secs - link_mtime_secs).unsigned_abs();
            assert!(
                diff <= 1,
                "symlink mtime in archive ({archived_secs}) should match symlink's own mtime ({link_mtime_secs})"
            );
            found_symlink = true;
        }
    })
    .unwrap();
    assert!(found_symlink, "no symlink entry found in archive");
}

/// Precondition: A broken symlink (pointing to a nonexistent target) exists.
/// Action: Run `pna create` with `--keep-timestamp --keep-permission`.
/// Expectation: The archive contains a SymbolicLink entry with permission and timestamp metadata.
#[test]
fn create_broken_symlink_stores_metadata() {
    setup();
    let base = "create_broken_symlink_stores_metadata";
    let source = format!("{base}/source");
    init_broken_symlink_resource(&source);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--keep-dir",
        "--keep-timestamp",
        "--keep-permission",
        "--unstable",
        &source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found_broken = false;
    archive::for_each_entry(format!("{base}/{base}.pna"), |entry| {
        if entry.header().data_kind() == pna::DataKind::SymbolicLink {
            assert_eq!(archive::read_symlink_target(&entry), "nonexistent");
            // wasi has no POSIX mode; permission is never populated there.
            #[cfg(not(target_os = "wasi"))]
            {
                assert!(
                    entry.metadata().permission().is_some(),
                    "broken symlink should have permission metadata"
                );
                assert!(
                    entry.metadata().modified().is_some(),
                    "broken symlink should have mtime metadata"
                );
            }
            found_broken = true;
        }
    })
    .unwrap();
    assert!(found_broken, "no broken symlink entry found in archive");
}

/// Precondition: A regular file and a symlink pointing to it exist.
/// Action: Run `pna create` with `--follow-links --keep-permission`.
/// Expectation: The symlink is archived as a regular file with the target's permission.
#[cfg(unix)]
#[test]
fn create_follow_links_stores_target_metadata() {
    use std::os::unix::fs::PermissionsExt;

    setup();
    let base = "create_follow_links_stores_target_metadata";
    let source = format!("{base}/source");
    init_symlink_resource(&source);

    // Set target file to a known permission
    fs::set_permissions(
        format!("{source}/target.txt"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--keep-dir",
        "--keep-permission",
        "--unstable",
        "--follow-links",
        &source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut link_entry_kind = None;
    let mut link_entry_perm = None;
    archive::for_each_entry(format!("{base}/{base}.pna"), |entry| {
        if entry.header().path().as_str().ends_with("link.txt") {
            link_entry_kind = Some(entry.header().data_kind());
            link_entry_perm = entry.metadata().permission().map(|p| p.permissions());
        }
    })
    .unwrap();

    assert_eq!(
        link_entry_kind,
        Some(pna::DataKind::File),
        "with --follow-links, symlink should be archived as File"
    );
    assert_eq!(
        link_entry_perm.map(|p| p & 0o777),
        Some(0o755),
        "with --follow-links, archived permission should be the target's"
    );
}

/// Precondition: A regular file with a known mtime and a symlink pointing to it exist.
/// Action: Run `pna create` with `--follow-links --keep-timestamp`.
/// Expectation: The symlink is archived as a regular file with the target's mtime.
#[test]
fn create_follow_links_stores_target_timestamps() {
    setup();
    let base = "create_follow_links_stores_target_timestamps";
    let source = format!("{base}/source");
    init_symlink_resource(&source);

    // Get the target file's mtime
    let target_meta = fs::metadata(format!("{source}/target.txt")).unwrap();
    let target_mtime_secs = target_meta
        .modified()
        .unwrap()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--keep-dir",
        "--keep-timestamp",
        "--follow-links",
        &source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut link_entry_kind = None;
    let mut link_entry_mtime = None;
    archive::for_each_entry(format!("{base}/{base}.pna"), |entry| {
        if entry.header().path().as_str().ends_with("link.txt") {
            link_entry_kind = Some(entry.header().data_kind());
            link_entry_mtime = entry.metadata().modified().map(|d| d.whole_seconds());
        }
    })
    .unwrap();

    assert_eq!(
        link_entry_kind,
        Some(pna::DataKind::File),
        "with --follow-links, symlink should be archived as File"
    );
    let archived_secs = link_entry_mtime.expect("should have mtime");
    let diff = (archived_secs - target_mtime_secs).unsigned_abs();
    assert!(
        diff <= 1,
        "with --follow-links, archived mtime should be the target's"
    );
}

/// Precondition: A target file with permission 0o600 and a symlink pointing to it exist.
/// Action: Run `pna create` with `--keep-permission` (no --follow-links).
/// Expectation: The symlink entry's permission reflects the symlink itself, not the target's 0o600.
#[cfg(unix)]
#[test]
fn create_symlink_does_not_store_target_permissions() {
    use std::os::unix::fs::PermissionsExt;

    setup();
    let base = "create_symlink_does_not_store_target_permissions";
    let source = format!("{base}/source");
    init_symlink_resource(&source);

    // Set target to a restrictive permission (0o600) that differs from symlink mode on all platforms
    // (Linux symlinks are 0o777, macOS symlinks are 0o755 — both differ from 0o600)
    fs::set_permissions(
        format!("{source}/target.txt"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--keep-dir",
        "--keep-permission",
        "--unstable",
        &source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut target_perm = None;
    let mut symlink_perm = None;
    archive::for_each_entry(format!("{base}/{base}.pna"), |entry| {
        if entry.header().path().as_str().ends_with("target.txt") {
            target_perm = entry
                .metadata()
                .permission()
                .map(|p| p.permissions() & 0o777);
        }
        if entry.header().data_kind() == pna::DataKind::SymbolicLink {
            symlink_perm = entry
                .metadata()
                .permission()
                .map(|p| p.permissions() & 0o777);
        }
    })
    .unwrap();

    assert_eq!(target_perm, Some(0o600), "target file should have 0o600");
    // Symlink mode differs from target's: Linux=0o777, macOS=0o755 — neither is 0o600.
    assert_ne!(
        symlink_perm, target_perm,
        "symlink permission should differ from target's (symlink's own mode, not target's)"
    );
}

/// Precondition: A directory contains a regular file and a symlink pointing to it.
/// Action: Run `pna create` then `pna extract` with `--keep-permission --keep-timestamp`.
/// Expectation: The extracted symlink exists and points to the correct target.
#[test]
fn roundtrip_symlink_metadata_preserved() {
    setup();
    let base = "roundtrip_symlink_metadata_preserved";
    let source = format!("{base}/source");
    init_symlink_resource(&source);

    // Create archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--keep-dir",
        "--keep-permission",
        "--unstable",
        "--keep-timestamp",
        &source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify the archive has a symlink entry with metadata
    let mut has_symlink_with_perm = false;
    archive::for_each_entry(format!("{base}/{base}.pna"), |entry| {
        if entry.header().data_kind() == pna::DataKind::SymbolicLink {
            // wasi has no POSIX mode; permission is never populated there.
            #[cfg(not(target_os = "wasi"))]
            {
                assert!(entry.metadata().permission().is_some());
                assert!(entry.metadata().modified().is_some());
            }
            has_symlink_with_perm = true;
        }
    })
    .unwrap();
    assert!(has_symlink_with_perm);

    // Extract
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--out-dir",
        &format!("{base}/dist"),
        "--keep-permission",
        "--unstable",
        "--keep-timestamp",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link_path = PathBuf::from(format!("{base}/dist/link.txt"));
    assert!(link_path.is_symlink(), "extracted path should be a symlink");
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("target.txt"),
    );
    // Target file should also be extracted correctly
    let content = fs::read_to_string(format!("{base}/dist/target.txt")).unwrap();
    assert_eq!(content, "content");
}

/// Precondition: A directory contains a broken symlink.
/// Action: Run `pna create` then `pna extract` with `--keep-permission`.
/// Expectation: The broken symlink is preserved with correct link target through the roundtrip.
#[test]
fn roundtrip_broken_symlink_metadata_preserved() {
    setup();
    let base = "roundtrip_broken_symlink_metadata_preserved";
    let source = format!("{base}/source");
    init_broken_symlink_resource(&source);

    // Create archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--keep-dir",
        "--keep-permission",
        "--unstable",
        &source,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive content
    let mut has_broken_symlink = false;
    archive::for_each_entry(format!("{base}/{base}.pna"), |entry| {
        if entry.header().data_kind() == pna::DataKind::SymbolicLink {
            assert_eq!(archive::read_symlink_target(&entry), "nonexistent");
            // wasi has no POSIX mode; permission is never populated there.
            #[cfg(not(target_os = "wasi"))]
            assert!(entry.metadata().permission().is_some());
            has_broken_symlink = true;
        }
    })
    .unwrap();
    assert!(has_broken_symlink);

    // Extract
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{base}/{base}.pna"),
        "--overwrite",
        "--out-dir",
        &format!("{base}/dist"),
        "--keep-permission",
        "--unstable",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link_path = PathBuf::from(format!("{base}/dist/broken.txt"));
    assert!(
        link_path.is_symlink(),
        "broken symlink should survive roundtrip"
    );
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("nonexistent"),
    );
}
