#![cfg(not(target_family = "wasm"))]
//! Tests for `-p, --same-permissions` and related permission flags in stdio mode.
//!
//! Phase 1: Creation defaults (mode+owner stored by default)
//! Phase 2: Extraction with -p flag (restores mode+ACL+xattr+fflags+mac-meta, NOT owner)

use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::predicate;
use std::fs;

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
