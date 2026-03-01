# CLI Integration Test Invariants — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Strengthen CLI integration test assertions so that passing tests reliably prove the archiver preserves all file attributes through round-trips.

**Architecture:** Incremental assertion strengthening — add missing assertions to existing tests without restructuring. Two-layer verification: Layer 1 (archive-level, environment-independent) is mandatory; Layer 2 (filesystem-level) uses `#[cfg]` gating.

**Tech Stack:** Rust test framework, `pna` crate API (`NormalEntry`, `ReadOptions`), `std::fs` for Layer 2 assertions.

**Design doc:** `docs/plans/2026-02-23-cli-integration-test-invariants-design.md`

---

## Phase 1: HIGH Priority — Missing Verification

### Task 1: I-4 Symlink target verification in `symlink_no_follow`

**Files:**
- Modify: `cli/tests/cli/create/symlink.rs:44-112` (test `symlink_no_follow`)

**Step 1: Add Layer 1 (archive-level) symlink target assertions**

In the `archive::for_each_entry` closure, add target path verification for SymbolicLink entries. Symlink targets are stored as entry data — read via `entry.reader()`.

Add to the match arms for SymbolicLink entries (lines 76-84):

```rust
"symlink_no_follow/source/dir/in_dir_link.txt" => {
    assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
    // I-4: Verify symlink target is preserved in archive
    let mut target = Vec::new();
    entry.reader(pna::ReadOptions::with_password::<&[u8]>(None)).unwrap()
        .read_to_end(&mut target).unwrap();
    assert_eq!(String::from_utf8(target).unwrap(), "in_dir_text.txt");
}
"symlink_no_follow/source/link_dir" => {
    assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
    let mut target = Vec::new();
    entry.reader(pna::ReadOptions::with_password::<&[u8]>(None)).unwrap()
        .read_to_end(&mut target).unwrap();
    assert_eq!(String::from_utf8(target).unwrap(), "dir");
}
"symlink_no_follow/source/link.txt" => {
    assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
    let mut target = Vec::new();
    entry.reader(pna::ReadOptions::with_password::<&[u8]>(None)).unwrap()
        .read_to_end(&mut target).unwrap();
    assert_eq!(String::from_utf8(target).unwrap(), "text.txt");
}
```

Add `use std::io::Read;` to the file imports if not present.

**Step 2: Add Layer 2 (filesystem-level) symlink target assertions**

After extraction (after line 103), add `fs::read_link()` assertions:

```rust
// I-4: Verify symlink targets on filesystem
assert_eq!(
    fs::read_link("symlink_no_follow/dist/link.txt").unwrap(),
    Path::new("text.txt"),
);
assert_eq!(
    fs::read_link("symlink_no_follow/dist/link_dir").unwrap(),
    Path::new("dir"),
);
assert_eq!(
    fs::read_link("symlink_no_follow/dist/dir/in_dir_link.txt").unwrap(),
    Path::new("in_dir_text.txt"),
);
```

**Step 3: Run test to verify it passes**

Run: `cargo test -p portable-network-archive --test cli -- symlink_no_follow --exact`
Expected: PASS

**Step 4: Commit**

```bash
git add cli/tests/cli/create/symlink.rs
git commit -m ":white_check_mark: Add symlink target verification to symlink_no_follow test"
```

---

### Task 2: I-4 Symlink target verification in `broken_symlink_no_follow`

**Files:**
- Modify: `cli/tests/cli/create/symlink.rs:200-252` (test `broken_symlink_no_follow`)

**Step 1: Add Layer 1 symlink target assertions for broken symlinks**

In the `archive::for_each_entry` closure, add target verification:

```rust
"broken_symlink_no_follow/source/broken.txt" => {
    assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
    let mut target = Vec::new();
    entry.reader(pna::ReadOptions::with_password::<&[u8]>(None)).unwrap()
        .read_to_end(&mut target).unwrap();
    assert_eq!(String::from_utf8(target).unwrap(), "missing.txt");
}
"broken_symlink_no_follow/source/broken_dir" => {
    assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
    let mut target = Vec::new();
    entry.reader(pna::ReadOptions::with_password::<&[u8]>(None)).unwrap()
        .read_to_end(&mut target).unwrap();
    assert_eq!(String::from_utf8(target).unwrap(), "missing_dir");
}
```

**Step 2: Add Layer 2 symlink target assertions after extraction**

```rust
// I-4: Verify broken symlink targets are preserved
assert_eq!(
    fs::read_link("broken_symlink_no_follow/dist/broken.txt").unwrap(),
    Path::new("missing.txt"),
);
assert_eq!(
    fs::read_link("broken_symlink_no_follow/dist/broken_dir").unwrap(),
    Path::new("missing_dir"),
);
```

**Step 3: Run test**

Run: `cargo test -p portable-network-archive --test cli -- broken_symlink_no_follow --exact`
Expected: PASS

**Step 4: Commit**

```bash
git add cli/tests/cli/create/symlink.rs
git commit -m ":white_check_mark: Add symlink target verification to broken_symlink_no_follow test"
```

---

### Task 3: I-4 Symlink target verification in remaining symlink tests

**Files:**
- Modify: `cli/tests/cli/create/symlink.rs` — tests `broken_symlink_follow`, `symlink_depth0_no_follow_file`, `symlink_depth0_no_follow_dir`

**Step 1: Add Layer 1 + Layer 2 assertions to `broken_symlink_follow`**

Same pattern as Task 2. Broken symlinks with `--follow-links` remain symlinks, so targets should be `"missing.txt"` and `"missing_dir"`.

**Step 2: Add to `symlink_depth0_no_follow_file`**

Layer 1: In `for_each_entry` closure, verify target is `"target.txt"`.
Layer 2: After extraction, add:

```rust
assert_eq!(
    fs::read_link("symlink_depth0_no_follow_file/dist/link.txt").unwrap(),
    Path::new("target.txt"),
);
```

**Step 3: Add to `symlink_depth0_no_follow_dir`**

Layer 1: In `for_each_entry` closure, verify target is `"dir"`.
Layer 2: After extraction, add:

```rust
assert_eq!(
    fs::read_link("symlink_depth0_no_follow_dir/dist/link_dir").unwrap(),
    Path::new("dir"),
);
```

**Step 4: Run all symlink tests**

Run: `cargo test -p portable-network-archive --test cli -- symlink`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add cli/tests/cli/create/symlink.rs
git commit -m ":white_check_mark: Add symlink target verification to remaining symlink tests"
```

---

### Task 4: I-7 Ownership preservation — extract-side verification

**Files:**
- Modify: `cli/tests/cli/create/user_group.rs`

Currently, `archive_create_uname_gname` and `archive_create_uid_gid` verify ownership
after create (archive inspection), then extract and verify content via `diff()`.
The gap: no verification that extracted archive still contains the ownership data.

**Step 1: Add Layer 1 archive re-read after extract in `archive_create_uname_gname`**

After the extract command and `diff()` call, add archive-level verification of the
created archive (the archive itself was already verified, but this documents the
invariant pattern — the ownership data survived the create pipeline):

The existing test already verifies ownership in archive after create. The gap is that
`diff()` doesn't verify ownership on extracted files (which is correct — ownership on
FS requires root). The existing Layer 1 verification is actually sufficient here.

**Action**: Add assertion messages to existing ownership checks for clarity, and verify
the extracted archive's content byte-completeness (replace `diff()` with direct `fs::read` assertions).

Replace the `diff()` call in both tests with direct byte comparisons:

For `archive_create_uname_gname`:
```rust
// I-1: Verify byte completeness (replaces diff)
assert_eq!(
    fs::read("archive_create_uname_gname/out/raw/text.txt").unwrap(),
    fs::read("archive_create_uname_gname/in/raw/text.txt").unwrap(),
);
assert_eq!(
    fs::read("archive_create_uname_gname/out/raw/empty.txt").unwrap(),
    fs::read("archive_create_uname_gname/in/raw/empty.txt").unwrap(),
);
```

For `archive_create_uid_gid`:
```rust
assert_eq!(
    fs::read("archive_create_uid_gid/out/raw/text.txt").unwrap(),
    fs::read("archive_create_uid_gid/in/raw/text.txt").unwrap(),
);
assert_eq!(
    fs::read("archive_create_uid_gid/out/raw/empty.txt").unwrap(),
    fs::read("archive_create_uid_gid/in/raw/empty.txt").unwrap(),
);
```

Add `use std::fs;` to imports.

Remove the `diff::diff` import if no longer used in this file.

**Step 2: Run tests**

Run: `cargo test -p portable-network-archive --test cli -- archive_create_u`
Expected: PASS

**Step 3: Commit**

```bash
git add cli/tests/cli/create/user_group.rs
git commit -m ":white_check_mark: Replace diff() with direct byte assertions in ownership tests"
```

---

### Task 5: I-8 Xattr preservation — round-trip verification

**Files:**
- Modify: `cli/tests/cli/xattr/set/basic.rs`

Currently, xattr tests verify that `pna xattr set` stores xattrs in the archive correctly.
The gap: no round-trip test (create with xattr → extract → verify xattr survives).

**Step 1: Add a round-trip xattr preservation test**

Add a new test function after `xattr_empty_value`:

```rust
/// Precondition: An archive entry has extended attributes set.
/// Action: Extract the archive with `--keep-xattr`, then re-create a new archive from extracted files.
/// Expectation: The xattr data in the new archive matches the original.
#[test]
fn xattr_round_trip_preservation() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_roundtrip/").unwrap();

    // Set xattr on an entry
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_roundtrip/zstd.pna",
        "--name",
        "user.roundtrip",
        "--value",
        "preserved_value",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Layer 1: Verify xattr is stored in archive
    archive::for_each_entry("xattr_roundtrip/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.roundtrip".into(),
                    b"preserved_value".into()
                )]
            );
        }
    })
    .unwrap();

    // Extract with --keep-xattr
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "xattr_roundtrip/zstd.pna",
        "--overwrite",
        "--out-dir",
        "xattr_roundtrip/out/",
        "--keep-xattr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Re-create archive from extracted files with --keep-xattr
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_roundtrip/roundtrip.pna",
        "--overwrite",
        "xattr_roundtrip/out/",
        "--keep-xattr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Layer 1: Verify xattr survived the round-trip
    let mut found = false;
    archive::for_each_entry("xattr_roundtrip/roundtrip.pna", |entry| {
        if entry.name().as_str().ends_with("raw/empty.txt") {
            found = true;
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.roundtrip".into(),
                    b"preserved_value".into()
                )],
            );
        }
    })
    .unwrap();
    assert!(found, "raw/empty.txt entry not found in round-trip archive");
}
```

**Step 2: Run test**

Run: `cargo test -p portable-network-archive --test cli -- xattr_round_trip_preservation --exact`
Expected: PASS (if xattr round-trip works) or FAIL (revealing a real bug)

**Step 3: Commit**

```bash
git add cli/tests/cli/xattr/set/basic.rs
git commit -m ":white_check_mark: Add xattr round-trip preservation test"
```

---

## Phase 2: MEDIUM Priority — Weak Verification

### Task 6: I-2/I-3 Structure and type assertions in `keep_all`

**Files:**
- Modify: `cli/tests/cli/keep_all.rs`

The `archive_keep_all` test uses only `diff()` for verification. Add direct assertions.

**Step 1: Add Layer 1 archive content assertions**

After the extract command and before the `diff()` call, add:

```rust
// I-2/I-3: Verify archive structure and types
let mut entry_paths = std::collections::HashSet::new();
archive::for_each_entry("archive_keep_all/keep_all.pna", |entry| {
    entry_paths.insert(entry.header().path().to_string());
})
.unwrap();
// Verify key entries exist (structure completeness)
assert!(
    entry_paths.iter().any(|p| p.ends_with("raw/text.txt")),
    "text.txt should be in archive"
);
assert!(
    entry_paths.iter().any(|p| p.ends_with("raw/empty.txt")),
    "empty.txt should be in archive"
);
assert!(
    entry_paths.iter().any(|p| p.ends_with("raw/images/icon.png")),
    "icon.png should be in archive"
);
```

**Step 2: Add Layer 2 filesystem assertions after extract**

```rust
// I-1: Verify byte completeness (not relying on diff alone)
assert_eq!(
    fs::read("archive_keep_all/out/raw/text.txt").unwrap(),
    fs::read("archive_keep_all/in/raw/text.txt").unwrap(),
);
assert_eq!(
    fs::read("archive_keep_all/out/raw/empty.txt").unwrap(),
    fs::read("archive_keep_all/in/raw/empty.txt").unwrap(),
);
assert_eq!(
    fs::read("archive_keep_all/out/raw/images/icon.png").unwrap(),
    fs::read("archive_keep_all/in/raw/images/icon.png").unwrap(),
);

// I-2: Verify directory structure
assert!(Path::new("archive_keep_all/out/raw").is_dir());
assert!(Path::new("archive_keep_all/out/raw/images").is_dir());

// I-3: Verify file types
assert!(Path::new("archive_keep_all/out/raw/text.txt").is_file());
assert!(Path::new("archive_keep_all/out/raw/empty.txt").is_file());
assert!(Path::new("archive_keep_all/out/raw/images/icon.png").is_file());
```

Add `use std::path::Path;` to imports.

**Step 3: Run test**

Run: `cargo test -p portable-network-archive --test cli -- archive_keep_all --exact`
Expected: PASS

**Step 4: Commit**

```bash
git add cli/tests/cli/keep_all.rs
git commit -m ":white_check_mark: Add direct structure and content assertions to keep_all test"
```

---

### Task 7: I-5 Directory permission preservation

**Files:**
- Modify: `cli/tests/cli/extract/option_keep_permission.rs`

Currently tests only verify file permissions. Directories are not verified.

**Step 1: Add directory permission test**

Add a new test after `extract_preserves_mixed_permissions`:

```rust
/// Precondition: An archive contains a directory with permission 0o750 (rwxr-x---).
/// Action: Extract with `--keep-permission`.
/// Expectation: The extracted directory has permission 0o750 on the filesystem.
#[test]
#[cfg(unix)]
fn extract_preserves_directory_permission() {
    setup();
    TestResources::extract_in("raw/", "extract_dir_perm/in/").unwrap();

    set_permissions_or_skip!("extract_dir_perm/in/raw/images", 0o750);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_dir_perm/archive.pna",
        "--overwrite",
        "extract_dir_perm/in/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // I-5 Layer 1: Verify directory permission in archive
    archive::for_each_entry("extract_dir_perm/archive.pna", |entry| {
        if entry.header().path().as_str().ends_with("raw/images") {
            assert_eq!(entry.header().data_kind(), pna::DataKind::Directory);
            let perm = entry.metadata().permission().unwrap();
            assert_eq!(
                perm.permissions() & 0o777,
                0o750,
                "directory should have 0o750 in archive"
            );
        }
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_dir_perm/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_dir_perm/out/",
        "--keep-permission",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // I-5 Layer 2: Verify directory permission on filesystem
    let meta = fs::symlink_metadata("extract_dir_perm/out/raw/images").unwrap();
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o750,
        "extracted directory should have permission 0o750"
    );
}
```

Add `use crate::utils::archive;` to imports if not present.

**Step 2: Run test**

Run: `cargo test -p portable-network-archive --test cli -- extract_preserves_directory_permission --exact`
Expected: PASS

**Step 3: Commit**

```bash
git add cli/tests/cli/extract/option_keep_permission.rs
git commit -m ":white_check_mark: Add directory permission preservation test"
```

---

### Task 8: I-5 Layer 1 assertions in existing permission tests

**Files:**
- Modify: `cli/tests/cli/extract/option_keep_permission.rs`

Existing file permission tests only check Layer 2 (filesystem). Add Layer 1 (archive) assertions.

**Step 1: Add archive-level permission verification to `extract_preserves_executable_permission`**

After the create command (line 82) and before the extract command (line 84), add:

```rust
// I-5 Layer 1: Verify permission stored correctly in archive
archive::for_each_entry("extract_perm_755/archive.pna", |entry| {
    if entry.header().path().as_str().ends_with("raw/text.txt") {
        let perm = entry.metadata().permission().unwrap();
        assert_eq!(
            perm.permissions() & 0o777,
            0o755,
            "archive should store permission 0o755"
        );
    }
})
.unwrap();
```

**Step 2: Apply same pattern to other permission tests**

Add Layer 1 assertions to: `extract_preserves_readonly_permission` (0o644),
`extract_preserves_private_permission` (0o600), `extract_preserves_full_permission` (0o777),
and `extract_preserves_mixed_permissions` (multiple values).

Skip `extract_preserves_no_permission` — the 0o000 test already has special permission handling.

**Step 3: Run all permission tests**

Run: `cargo test -p portable-network-archive --test cli -- extract_preserves`
Expected: ALL PASS

**Step 4: Commit**

```bash
git add cli/tests/cli/extract/option_keep_permission.rs
git commit -m ":white_check_mark: Add Layer 1 archive assertions to permission tests"
```

---

### Task 9: I-9 ACL archive-level verification

**Files:**
- Modify: `cli/tests/cli/keep_acl.rs`
- Modify: `cli/tests/cli/restore_acl.rs`

**Step 1: Add archive structure verification to `keep_acl.rs`**

In `archive_keep_acl`, after create and before extract, add:

```rust
// I-9 Layer 1: Verify archive contains expected entries
let mut entry_paths = std::collections::HashSet::new();
archive::for_each_entry("keep_acl/keep_acl.pna", |entry| {
    entry_paths.insert(entry.header().path().to_string());
})
.unwrap();
assert!(
    !entry_paths.is_empty(),
    "archive should contain entries"
);
```

Add `use crate::utils::archive;` to imports.

**Step 2: Add execution success assertions to `restore_acl.rs`**

The current `restore_acl.rs` tests only call `.unwrap()` — they don't verify anything
was actually extracted. Add existence checks:

For each test (e.g., `extract_linux_acl`), after execute, add:

```rust
assert!(
    std::path::Path::new("linux_acl/out/").exists(),
    "output directory should exist after extraction"
);
```

**Step 3: Run ACL tests**

Run: `cargo test -p portable-network-archive --all-features --test cli -- acl`
Expected: PASS

**Step 4: Commit**

```bash
git add cli/tests/cli/keep_acl.rs cli/tests/cli/restore_acl.rs
git commit -m ":white_check_mark: Add archive-level assertions to ACL tests"
```

---

## Phase 3: LOW Priority — Supplementary

### Task 10: I-1/I-2/I-3 Empty file and empty directory round-trip

**Files:**
- Create: `cli/tests/cli/create/empty_entries.rs`
- Modify: `cli/tests/cli/create.rs` (add `mod empty_entries;`)

**Step 1: Create test file**

```rust
use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{
    collections::HashSet,
    fs,
    path::Path,
};

/// Precondition: Source tree contains an empty file and an empty directory.
/// Action: Create archive, then extract.
/// Expectation: Empty file and empty directory survive round-trip.
#[test]
fn empty_file_and_directory_round_trip() {
    setup();
    let base = "empty_entries_roundtrip";
    if Path::new(base).exists() {
        fs::remove_dir_all(base).unwrap();
    }
    fs::create_dir_all(format!("{base}/source")).unwrap();

    // Create empty file
    fs::write(format!("{base}/source/empty.txt"), b"").unwrap();
    // Create empty directory
    fs::create_dir_all(format!("{base}/source/empty_dir")).unwrap();
    // Create non-empty file for comparison
    fs::write(format!("{base}/source/data.txt"), b"hello").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{base}/archive.pna"),
        "--overwrite",
        "--keep-dir",
        &format!("{base}/source"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    // I-2/I-3 Layer 1: Verify all entries in archive
    let mut entries = HashSet::new();
    archive::for_each_entry(&format!("{base}/archive.pna"), |entry| {
        entries.insert((
            entry.header().path().to_string(),
            entry.header().data_kind(),
        ));
    })
    .unwrap();

    assert!(
        entries.iter().any(|(p, k)| p.ends_with("empty.txt") && *k == pna::DataKind::File),
        "empty file should be in archive as File"
    );
    assert!(
        entries.iter().any(|(p, k)| p.ends_with("empty_dir") && *k == pna::DataKind::Directory),
        "empty directory should be in archive as Directory"
    );
    assert!(
        entries.iter().any(|(p, k)| p.ends_with("data.txt") && *k == pna::DataKind::File),
        "data file should be in archive as File"
    );

    // Extract
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{base}/archive.pna"),
        "--overwrite",
        "--out-dir",
        &format!("{base}/dist"),
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // I-1 Layer 2: Verify empty file content
    assert_eq!(fs::read(format!("{base}/dist/empty.txt")).unwrap(), b"");
    // I-1 Layer 2: Verify non-empty file content
    assert_eq!(fs::read(format!("{base}/dist/data.txt")).unwrap(), b"hello");
    // I-2 Layer 2: Verify empty directory exists
    assert!(Path::new(&format!("{base}/dist/empty_dir")).is_dir());
    // I-3 Layer 2: Verify types
    assert!(Path::new(&format!("{base}/dist/empty.txt")).is_file());
    assert!(Path::new(&format!("{base}/dist/data.txt")).is_file());
}
```

**Step 2: Register module**

In `cli/tests/cli/create.rs`, add `mod empty_entries;` alongside existing modules.

**Step 3: Run test**

Run: `cargo test -p portable-network-archive --test cli -- empty_file_and_directory_round_trip --exact`
Expected: PASS

**Step 4: Commit**

```bash
git add cli/tests/cli/create/empty_entries.rs cli/tests/cli/create.rs
git commit -m ":white_check_mark: Add empty file and directory round-trip test"
```

---

## Verification

After all tasks, run the full test suite:

```bash
cargo test -p portable-network-archive --all-features
```

Expected: ALL PASS. If any test fails, it reveals a real bug — investigate before marking complete.

---

## Summary

| Task | Invariant | Layer | Target File |
|------|-----------|-------|-------------|
| 1 | I-4 Symlink target | L1+L2 | `create/symlink.rs` (symlink_no_follow) |
| 2 | I-4 Symlink target | L1+L2 | `create/symlink.rs` (broken_symlink_no_follow) |
| 3 | I-4 Symlink target | L1+L2 | `create/symlink.rs` (remaining tests) |
| 4 | I-7 Ownership | L1 | `create/user_group.rs` |
| 5 | I-8 Xattr | L1 | `xattr/set/basic.rs` |
| 6 | I-2/I-3 Structure/Type | L1+L2 | `keep_all.rs` |
| 7 | I-5 Dir Permission | L1+L2 | `extract/option_keep_permission.rs` |
| 8 | I-5 Permission | L1 | `extract/option_keep_permission.rs` |
| 9 | I-9 ACL | L1 | `keep_acl.rs`, `restore_acl.rs` |
| 10 | I-1/I-2/I-3 Empty entries | L1+L2 | `create/empty_entries.rs` (new) |
