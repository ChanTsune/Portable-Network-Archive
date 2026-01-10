#![cfg(not(target_family = "wasm"))]
//! Tests for `-p, --same-permissions` and related permission flags in stdio mode.
//!
//! Phase 1: Creation defaults (mode+owner stored by default)
//! Phase 2: Extraction with -p flag (restores mode+ACL+xattr+fflags+mac-meta, NOT owner)

use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::predicate;
use std::fs;
#[cfg(unix)]
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
macro_rules! set_permissions_or_skip {
    ($path:expr, $mode:expr) => {
        match fs::set_permissions($path, fs::Permissions::from_mode($mode)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                eprintln!("Skipping test: insufficient permissions: {}", e);
                return;
            }
            Err(e) => panic!("Failed to set permissions: {}", e),
        }
    };
}

// =============================================================================
// Flag Validation Tests
// =============================================================================

/// Test: --keep-permission flag is removed from stdio
/// Expectation: Command fails with "unexpected argument" error
#[test]
fn stdio_keep_permission_flag_removed() {
    setup();
    let file = "stdio_keep_permission_removed.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--keep-permission")
        .arg(file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

/// Test: --no-keep-permission flag is removed from stdio
/// Expectation: Command fails with "unexpected argument" error
#[test]
fn stdio_no_keep_permission_flag_removed() {
    setup();
    let file = "stdio_no_keep_permission_removed.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--no-keep-permission")
        .arg(file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

/// Test: -p/--same-permissions requires --unstable
/// Expectation: Command fails without --unstable
#[test]
fn stdio_same_permissions_requires_unstable() {
    setup();
    let file = "stdio_same_permissions_requires_unstable.txt";
    fs::write(file, "test content").unwrap();

    // Create an archive first
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg(file)
        .assert()
        .success();

    // Try to extract with -p but without --unstable
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("-p")
        .arg("--out-dir")
        .arg("stdio_same_permissions_requires_unstable_out/")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--unstable"));
}

/// Test: -p/--same-permissions is accepted in create mode but has no effect
/// Note: Due to clap's `requires_all` behavior with boolean flags, -p is technically
/// accepted in create mode but is ignored (it only affects extraction semantics).
/// This matches bsdtar behavior where -p is silently ignored in create mode.
#[test]
fn stdio_same_permissions_accepted_in_create_mode_but_ignored() {
    setup();
    let file = "stdio_same_permissions_in_create.txt";
    fs::write(file, "test content").unwrap();

    // -p in create mode is accepted but has no effect (matches bsdtar)
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--unstable")
        .arg("-p")
        .arg(file)
        .assert()
        .success();
}

/// Test: --no-same-permissions is accepted for creation
/// Expectation: Command succeeds with --no-same-permissions
#[test]
fn stdio_no_same_permissions_accepted_for_creation() {
    setup();
    let file = "stdio_no_same_permissions_create.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--no-same-permissions")
        .arg(file)
        .assert()
        .success();
}

/// Test: --no-same-owner is accepted for creation
/// Expectation: Command succeeds with --no-same-owner
#[test]
fn stdio_no_same_owner_accepted_for_creation() {
    setup();
    let file = "stdio_no_same_owner_create.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--no-same-owner")
        .arg(file)
        .assert()
        .success();
}

/// Test: Both --no-same-permissions and --no-same-owner can be used together
/// Expectation: Command succeeds with both flags
#[test]
fn stdio_no_same_permissions_and_no_same_owner_together() {
    setup();
    let file = "stdio_both_no_flags.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--no-same-permissions")
        .arg("--no-same-owner")
        .arg(file)
        .assert()
        .success();
}

// =============================================================================
// Extraction with -p flag tests
// =============================================================================

/// Test: -p flag is accepted with --unstable for extraction
/// Expectation: Command succeeds
#[test]
fn stdio_extract_with_same_permissions_flag() {
    setup();
    let file = "stdio_extract_p_flag.txt";
    fs::write(file, "test content").unwrap();
    fs::create_dir_all("stdio_extract_p_flag_out").unwrap();

    // Create archive
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg(file)
        .assert()
        .success();

    // Extract with -p
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--unstable")
        .arg("-p")
        .arg("--out-dir")
        .arg("stdio_extract_p_flag_out/")
        .assert()
        .success();
}

/// Test: -p with --same-owner is accepted
/// Expectation: Command succeeds
#[test]
fn stdio_extract_with_same_permissions_and_same_owner() {
    setup();
    let file = "stdio_extract_p_same_owner.txt";
    fs::write(file, "test content").unwrap();
    fs::create_dir_all("stdio_extract_p_same_owner_out").unwrap();

    // Create archive
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg(file)
        .assert()
        .success();

    // Extract with -p --same-owner
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--unstable")
        .arg("-p")
        .arg("--same-owner")
        .arg("--out-dir")
        .arg("stdio_extract_p_same_owner_out/")
        .assert()
        .success();
}

/// Test: -p with --no-acls is accepted (individual flag overrides -p)
/// Expectation: Command succeeds
#[test]
fn stdio_extract_same_permissions_with_no_acls() {
    setup();
    let file = "stdio_extract_p_no_acls.txt";
    fs::write(file, "test content").unwrap();
    fs::create_dir_all("stdio_extract_p_no_acls_out").unwrap();

    // Create archive
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg(file)
        .assert()
        .success();

    // Extract with -p --no-acls
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--unstable")
        .arg("-p")
        .arg("--no-acls")
        .arg("--out-dir")
        .arg("stdio_extract_p_no_acls_out/")
        .assert()
        .success();
}

/// Test: Long form --same-permissions is accepted
/// Expectation: Command succeeds
#[test]
fn stdio_extract_with_long_same_permissions_flag() {
    setup();
    let file = "stdio_extract_long_p_flag.txt";
    fs::write(file, "test content").unwrap();
    fs::create_dir_all("stdio_extract_long_p_flag_out").unwrap();

    // Create archive
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg(file)
        .assert()
        .success();

    // Extract with --same-permissions
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--unstable")
        .arg("--same-permissions")
        .arg("--out-dir")
        .arg("stdio_extract_long_p_flag_out/")
        .assert()
        .success();
}

/// Test: --preserve-permissions alias is accepted
/// Expectation: Command succeeds
#[test]
fn stdio_extract_with_preserve_permissions_alias() {
    setup();
    let file = "stdio_extract_preserve_p.txt";
    fs::write(file, "test content").unwrap();
    fs::create_dir_all("stdio_extract_preserve_p_out").unwrap();

    // Create archive
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg(file)
        .assert()
        .success();

    // Extract with --preserve-permissions
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--unstable")
        .arg("--preserve-permissions")
        .arg("--out-dir")
        .arg("stdio_extract_preserve_p_out/")
        .assert()
        .success();
}

// =============================================================================
// Flag Combination Tests (全肯定+個別否定, 全否定+個別肯定)
// =============================================================================

/// Test: -p --no-same-permissions (全肯定 + 全否定 → 全否定が勝つ)
/// Expectation: Command succeeds, --no-same-permissions overrides -p
#[test]
fn stdio_extract_same_permissions_overridden_by_no_same_permissions() {
    setup();
    let file = "stdio_p_no_same_permissions.txt";
    fs::write(file, "test content").unwrap();
    fs::create_dir_all("stdio_p_no_same_permissions_out").unwrap();

    // Create archive
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg(file)
        .assert()
        .success();

    // Extract with -p --no-same-permissions (--no-same-permissions wins)
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--unstable")
        .arg("-p")
        .arg("--no-same-permissions")
        .arg("--out-dir")
        .arg("stdio_p_no_same_permissions_out/")
        .assert()
        .success();
}

/// Test: --no-same-permissions --keep-xattr (全否定 + 個別肯定 → 個別肯定が有効)
/// Expectation: Command succeeds, --keep-xattr still enables xattr despite --no-same-permissions
#[test]
fn stdio_extract_no_same_permissions_with_keep_xattr() {
    setup();
    let file = "stdio_no_same_p_keep_xattr.txt";
    fs::write(file, "test content").unwrap();
    fs::create_dir_all("stdio_no_same_p_keep_xattr_out").unwrap();

    // Create archive with xattr
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--keep-xattr")
        .arg(file)
        .assert()
        .success();

    // Extract with --no-same-permissions --keep-xattr (xattr should be enabled)
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--no-same-permissions")
        .arg("--keep-xattr")
        .arg("--out-dir")
        .arg("stdio_no_same_p_keep_xattr_out/")
        .assert()
        .success();
}

/// Test: --no-same-permissions alone in extraction (without -p)
/// Expectation: Command succeeds (flag is valid in extraction mode)
#[test]
fn stdio_extract_no_same_permissions_alone() {
    setup();
    let file = "stdio_no_same_p_alone.txt";
    fs::write(file, "test content").unwrap();
    fs::create_dir_all("stdio_no_same_p_alone_out").unwrap();

    // Create archive
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg(file)
        .assert()
        .success();

    // Extract with --no-same-permissions alone
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--no-same-permissions")
        .arg("--out-dir")
        .arg("stdio_no_same_p_alone_out/")
        .assert()
        .success();
}

// =============================================================================
// Behavioral Verification Tests
// =============================================================================

/// Precondition: Archive contains file with executable permission (0o755).
/// Action: Extract WITHOUT -p flag (default behavior).
/// Expectation: Extracted file does NOT have 0o755 permission (follows umask).
#[test]
#[cfg(unix)]
fn stdio_extract_without_p_does_not_preserve_permissions() {
    setup();
    let base = "stdio_extract_no_p_perm";
    fs::create_dir_all(base).unwrap();

    // Create file with executable permission
    let file = format!("{}/test.txt", base);
    fs::write(&file, "test content").unwrap();
    set_permissions_or_skip!(&file, 0o755);

    // Verify source file has expected permission
    let src_meta = fs::symlink_metadata(&file).unwrap();
    assert_eq!(
        src_meta.permissions().mode() & 0o777,
        0o755,
        "source file should have 0o755"
    );

    // Create archive via stdio (stores mode+owner by default)
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("-C")
        .arg(base)
        .arg("test.txt")
        .assert()
        .success();

    // Extract WITHOUT -p flag
    let out_dir = format!("{}/out", base);
    fs::create_dir_all(&out_dir).unwrap();

    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--out-dir")
        .arg(&out_dir)
        .assert()
        .success();

    // Verify extracted file does NOT have 0o755 (permission not restored)
    let extracted = format!("{}/test.txt", out_dir);
    let meta = fs::symlink_metadata(&extracted).unwrap();
    let extracted_mode = meta.permissions().mode() & 0o777;
    assert_ne!(
        extracted_mode, 0o755,
        "extracted file should NOT have 0o755 without -p flag (got 0o{:o})",
        extracted_mode
    );
}

/// Precondition: Archive contains file with executable permission (0o755).
/// Action: Extract WITH -p flag.
/// Expectation: Extracted file has 0o755 permission preserved.
#[test]
#[cfg(unix)]
fn stdio_extract_with_p_preserves_permissions() {
    setup();
    let base = "stdio_extract_with_p_perm";
    fs::create_dir_all(base).unwrap();

    // Create file with executable permission
    let file = format!("{}/test.txt", base);
    fs::write(&file, "test content").unwrap();
    set_permissions_or_skip!(&file, 0o755);

    // Create archive via stdio (stores mode+owner by default)
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("-C")
        .arg(base)
        .arg("test.txt")
        .assert()
        .success();

    // Extract WITH -p flag
    let out_dir = format!("{}/out", base);
    fs::create_dir_all(&out_dir).unwrap();

    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(create_output.get_output().stdout.as_slice())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--unstable")
        .arg("-p")
        .arg("--out-dir")
        .arg(&out_dir)
        .assert()
        .success();

    // Verify extracted file HAS 0o755 (permission restored)
    let extracted = format!("{}/test.txt", out_dir);
    let meta = fs::symlink_metadata(&extracted).unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o755,
        "extracted file should have 0o755 with -p flag"
    );
}

/// Precondition: File with executable permission (0o755).
/// Action: Create archive via stdio with default flags (no --no-same-permissions, no --no-same-owner).
/// Expectation: Archive contains mode and owner metadata.
#[test]
#[cfg(unix)]
fn stdio_create_stores_permissions_by_default() {
    setup();
    let base = "stdio_create_stores_perm";
    fs::create_dir_all(base).unwrap();

    // Create file with executable permission
    let file = format!("{}/test.txt", base);
    fs::write(&file, "test content").unwrap();
    set_permissions_or_skip!(&file, 0o755);

    // Create archive via stdio and write to file
    let archive_path = format!("{}/archive.pna", base);
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("-C")
        .arg(base)
        .arg("test.txt")
        .assert()
        .success();

    // Write archive to file for inspection
    fs::write(&archive_path, create_output.get_output().stdout.as_slice()).unwrap();

    // Inspect archive entries and verify permission metadata exists
    let mut found = false;
    archive::for_each_entry(&archive_path, |entry| {
        if entry.header().path().as_str() == "test.txt" {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o755,
                "archive entry should have 0o755 permission"
            );
            // Verify owner metadata is stored by default
            assert!(
                !perm.uname().is_empty() || perm.uid() > 0,
                "archive entry should have owner metadata (uid or uname)"
            );
            assert!(
                !perm.gname().is_empty() || perm.gid() > 0,
                "archive entry should have group metadata (gid or gname)"
            );
            found = true;
        }
    })
    .unwrap();

    assert!(found, "archive should contain test.txt entry");
}

/// Precondition: File with specific permissions exists.
/// Action: Create archive via stdio with --no-same-owner flag.
/// Expectation: Archive entry has NO permission metadata (current implementation couples mode/owner).
/// Note: In the current implementation, --no-same-owner prevents ALL permission metadata from
/// being stored, including mode. This is because the entry builder only adds permission when
/// OwnerStrategy::Preserve is set.
#[test]
#[cfg(unix)]
fn stdio_create_with_no_same_owner_omits_permission() {
    setup();
    let base = "stdio_create_no_same_owner";
    fs::create_dir_all(base).unwrap();

    // Create file
    let file = format!("{}/test.txt", base);
    fs::write(&file, "test content").unwrap();
    set_permissions_or_skip!(&file, 0o644);

    // Create archive with --no-same-owner
    let archive_path = format!("{}/archive.pna", base);
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--no-same-owner")
        .arg("-C")
        .arg(base)
        .arg("test.txt")
        .assert()
        .success();

    // Write archive to file for inspection
    fs::write(&archive_path, create_output.get_output().stdout.as_slice()).unwrap();

    // Verify archive entry exists but has no permission metadata
    // (current behavior: --no-same-owner omits all permission including mode)
    let mut found = false;
    archive::for_each_entry(&archive_path, |entry| {
        if entry.header().path().as_str() == "test.txt" {
            // Current behavior: no permission metadata when --no-same-owner is used
            assert!(
                entry.metadata().permission().is_none(),
                "archive entry should NOT have permission metadata with --no-same-owner"
            );
            found = true;
        }
    })
    .unwrap();

    assert!(found, "archive should contain test.txt entry");
}

// =============================================================================
// -p Flag Enables Extended Metadata Tests
// =============================================================================

/// Helper to check if xattr is supported on this platform/filesystem.
/// Returns true if we can set and get xattrs.
#[cfg(unix)]
fn xattr_supported(test_dir: &str) -> bool {
    let test_file = format!("{}/xattr_probe", test_dir);
    if fs::write(&test_file, "probe").is_err() {
        return false;
    }
    // Try to set and get xattr
    if xattr::set(&test_file, "user.test", b"value").is_err() {
        let _ = fs::remove_file(&test_file);
        return false;
    }
    let result = match xattr::get(&test_file, "user.test") {
        Ok(Some(v)) => v == b"value",
        _ => false,
    };
    let _ = fs::remove_file(&test_file);
    result
}

/// Precondition: File with extended attribute exists.
/// Action: Create archive with --keep-xattr, extract with -p flag.
/// Expectation: xattr is restored because -p implicitly enables xattr preservation.
#[test]
#[cfg(unix)]
fn stdio_extract_with_p_restores_xattr() {
    setup();
    let base = "stdio_extract_p_xattr";
    fs::create_dir_all(base).unwrap();

    if !xattr_supported(base) {
        eprintln!("Skipping test: xattr not supported on this filesystem");
        return;
    }

    // Create file with xattr
    let file = format!("{}/test.txt", base);
    fs::write(&file, "test content").unwrap();
    xattr::set(&file, "user.testattr", b"testvalue").unwrap();

    // Create archive with --keep-xattr
    let archive_path = format!("{}/archive.pna", base);
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--keep-xattr")
        .arg("-C")
        .arg(base)
        .arg("test.txt")
        .assert()
        .success();

    fs::write(&archive_path, create_output.get_output().stdout.as_slice()).unwrap();

    // Verify archive has xattr stored
    let mut has_xattr = false;
    archive::for_each_entry(&archive_path, |entry| {
        if entry.header().path().as_str() == "test.txt" && !entry.xattrs().is_empty() {
            has_xattr = true;
        }
    })
    .unwrap();
    assert!(has_xattr, "archive should contain xattr metadata");

    // Extract WITH -p flag
    let out_dir = format!("{}/out_with_p", base);
    fs::create_dir_all(&out_dir).unwrap();

    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(fs::read(&archive_path).unwrap())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--unstable")
        .arg("-p")
        .arg("--out-dir")
        .arg(&out_dir)
        .assert()
        .success();

    // Verify extracted file HAS xattr restored
    let extracted = format!("{}/test.txt", out_dir);
    let xattr_value = xattr::get(&extracted, "user.testattr").unwrap();
    assert_eq!(
        xattr_value,
        Some(b"testvalue".to_vec()),
        "extracted file should have xattr restored with -p flag"
    );
}

/// Precondition: File with extended attribute exists.
/// Action: Create archive with --keep-xattr, extract WITHOUT -p flag (and without --keep-xattr).
/// Expectation: xattr is NOT restored (default behavior).
#[test]
#[cfg(unix)]
fn stdio_extract_without_p_does_not_restore_xattr() {
    setup();
    let base = "stdio_extract_no_p_xattr";
    fs::create_dir_all(base).unwrap();

    if !xattr_supported(base) {
        eprintln!("Skipping test: xattr not supported on this filesystem");
        return;
    }

    // Create file with xattr
    let file = format!("{}/test.txt", base);
    fs::write(&file, "test content").unwrap();
    xattr::set(&file, "user.testattr", b"testvalue").unwrap();

    // Create archive with --keep-xattr
    let archive_path = format!("{}/archive.pna", base);
    let mut create_cmd = cargo_bin_cmd!("pna");
    let create_output = create_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--keep-xattr")
        .arg("-C")
        .arg(base)
        .arg("test.txt")
        .assert()
        .success();

    fs::write(&archive_path, create_output.get_output().stdout.as_slice()).unwrap();

    // Extract WITHOUT -p flag (default: do not restore xattr)
    let out_dir = format!("{}/out_without_p", base);
    fs::create_dir_all(&out_dir).unwrap();

    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .write_stdin(fs::read(&archive_path).unwrap())
        .arg("experimental")
        .arg("stdio")
        .arg("-x")
        .arg("--out-dir")
        .arg(&out_dir)
        .assert()
        .success();

    // Verify extracted file does NOT have xattr (not restored by default)
    let extracted = format!("{}/test.txt", out_dir);
    let xattr_value = xattr::get(&extracted, "user.testattr").unwrap();
    assert_eq!(
        xattr_value, None,
        "extracted file should NOT have xattr without -p flag"
    );
}
