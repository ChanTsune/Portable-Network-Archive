# CLI Integration Test Invariants Design

## Problem

CLI integration tests have reasonable test case coverage, but assertions are weak.
Tests pass even when critical archiver invariants are violated. Following t-wada's
philosophy of "test reliability" — when tests are green, we must be confident the
archiver actually works correctly.

## Scope

Round-trip completeness for `create` + `extract` as the representative command pair.
All file attributes covered. Other commands (append, update, etc.) verify only their
unique responsibilities.

## Out of Scope

- Encryption test strengthening (already sufficient)
- Solid mode exhaustive testing (architecturally equivalent to Normal mode)
- `diff()` helper bug fix (separate concern)
- Hardlink inode verification

---

## Fundamental Principle

> **Round-trip idempotency**: Data does not change through archival.
> `extract(create(source)) ≡ source` (for all attributes)

## Invariants

| ID | Invariant | Description |
|----|-----------|-------------|
| I-1 | Byte completeness | File contents survive round-trip byte-for-byte |
| I-2 | Structure completeness | Directory tree is identical |
| I-3 | Type completeness | File/directory/symlink distinction preserved |
| I-4 | Symlink target | Link target path preserved |
| I-5 | Permission preservation | Unix mode bits (rwx) preserved |
| I-6 | Timestamp preservation | mtime/atime preserved |
| I-7 | Ownership preservation | uid/gid/uname/gname preserved |
| I-8 | Xattr preservation | Extended attributes preserved |
| I-9 | ACL preservation | Access control lists preserved |
| I-10 | Fflags preservation | File flags preserved |

### Layer Responsibility

- **I-1 through I-4**: MUST be verified in every round-trip test (foundational)
- **I-5 through I-10**: Verified when `--keep-*` options are used (conditional)

---

## Two-Layer Verification Model

Tests that depend on execution environment are disqualified. To ensure environment
independence, verification is split into two layers:

| Layer | What it verifies | Environment dependency |
|-------|-----------------|----------------------|
| **Layer 1: Archive-level** | Metadata correctly stored in archive entries | **None** — runs anywhere |
| **Layer 2: Filesystem-level** | Extracted file attributes on filesystem | **Yes** — requires `#[cfg]` gating |

**Strategy**: Layer 1 is the mandatory foundation for all tests. Layer 2 adds
filesystem verification behind appropriate `#[cfg]` gates.

### Verification Patterns Per Invariant

#### I-1: Byte completeness
```rust
// Layer 1: Read entry content from archive
let mut content = Vec::new();
entry.reader().read_to_end(&mut content).unwrap();
assert_eq!(content, expected_bytes);

// Layer 2: Compare filesystem files
assert_eq!(fs::read("out/file.txt").unwrap(), fs::read("in/file.txt").unwrap());
```

#### I-2: Structure completeness
```rust
// Layer 1: Verify entry set
let entries: HashSet<String> = /* collect entry paths */;
assert!(entries.contains("dir/"));
assert!(entries.contains("dir/file.txt"));

// Layer 2: Verify directory existence
assert!(Path::new("out/dir").is_dir());
```

#### I-3: Type completeness
```rust
// Layer 1: Verify DataKind in archive
assert_eq!(entry.header().data_kind(), DataKind::File);
assert_eq!(entry.header().data_kind(), DataKind::Directory);
assert_eq!(entry.header().data_kind(), DataKind::SymbolicLink);

// Layer 2: Verify on filesystem
assert!(Path::new("out/file.txt").is_file());
assert!(Path::new("out/dir").is_dir());
assert!(Path::new("out/link").is_symlink());
```

#### I-4: Symlink target (currently MISSING)
```rust
// Layer 1: Verify link target stored in archive entry
// (implementation depends on how symlink target is exposed in NormalEntry API)

// Layer 2: Verify on filesystem
assert_eq!(fs::read_link("out/link.txt").unwrap(), Path::new("target.txt"));
```

#### I-5: Permission preservation
```rust
// Layer 1: Verify in archive
let perm = entry.metadata().permission().unwrap();
assert_eq!(perm.permissions() & 0o777, 0o755);

// Layer 2: Verify on filesystem
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::symlink_metadata("out/file.txt").unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
}
```

#### I-6: Timestamp preservation
```rust
// Layer 1: Verify mtime stored in archive
let archive_mtime = entry.metadata().modified().unwrap();
assert_same_second(archive_mtime, expected_mtime, "mtime");

// Layer 2: Verify on filesystem
let fs_mtime = fs::metadata("out/file.txt").unwrap().modified().unwrap();
assert_same_second(fs_mtime, expected_mtime, "mtime");
```

#### I-7: Ownership preservation (Layer 1 ONLY — root required for Layer 2)
```rust
// Layer 1: Verify in archive
let perm = entry.metadata().permission().unwrap();
assert_eq!(perm.uid(), expected_uid);
assert_eq!(perm.gid(), expected_gid);
```

#### I-8: Xattr preservation
```rust
// Layer 1: Verify in archive entry metadata
// (inspect xattr data from entry)

// Layer 2: Verify on filesystem
#[cfg(unix)]
{
    let val = xattr::get("out/file.txt", "user.test").unwrap();
    assert_eq!(val, Some(b"value".to_vec()));
}
```

#### I-9: ACL preservation
```rust
// Layer 1: Verify in archive entry metadata
#[cfg(feature = "acl")]
{
    // inspect ACL data from entry
}
```

#### I-10: Fflags preservation (Layer 1 ONLY — BSD/macOS specific)
```rust
// Layer 1: Verify in archive entry metadata
// (inspect fflags from entry)
```

---

## Programmatic Archive Creation

Use `archive::create_archive_with_permissions()` and similar helpers to create archives
with known metadata, bypassing filesystem constraints. This enables environment-independent
testing of metadata round-trips.

---

## Gap Analysis and Strengthening Plan

### Phase 1: HIGH Priority (Missing Verification)

| Target | Gap | Action |
|--------|-----|--------|
| `create/symlink.rs` | No `read_link()` target verification | Add I-4 Layer 1 + Layer 2 assertions |
| `create/user_group.rs` | No extract-side ownership verification | Add I-7 Layer 1 assertions (archive re-read after create) |
| `xattr.rs` | No extract-side xattr verification | Add I-8 Layer 1 + Layer 2 assertions |

### Phase 2: MEDIUM Priority (Weak Verification)

| Target | Gap | Action |
|--------|-----|--------|
| Round-trip tests using only `diff()` | Content/structure not directly asserted | Add I-2/I-3 direct assertions alongside `diff()` |
| `extract/option_keep_permission.rs` | Directory permissions not tested | Add I-5 directory permission assertions |
| `keep_acl.rs`, `restore_acl.rs` | No extract-side ACL verification | Add I-9 Layer 1 assertions |

### Phase 3: LOW Priority (Supplementary)

| Target | Gap | Action |
|--------|-----|--------|
| fflags tests | Only output display tested | Add I-10 Layer 1 assertions |
| Edge cases | No empty file/directory round-trip test | Add basic round-trip test |

---

## Principles

1. **No new use of `diff()`** — known bugs, use direct assertions instead
2. **Layer 1 is mandatory** — archive-level verification works everywhere
3. **Layer 2 is `#[cfg]`-gated** — never fail due to missing platform features
4. **Existing tests are additive** — add assertions, don't restructure working tests
5. **Environment-dependent tests are disqualified** — never require root, specific OS, or FS features without proper gating
