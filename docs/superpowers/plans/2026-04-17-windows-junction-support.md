# Windows Junction Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect Windows junctions during PNA archive creation and restore them during extraction, using the `DataKind::HardLink + fLTP=Directory` encoding with an external target path (absolute on CREATE; both absolute and relative accepted on EXTRACT for forward compatibility).

**Architecture:** No change to `libpna` or `pna` (`pna` stays platform-dependency-free). All Windows reparse-point FFI primitives live in `cli/src/utils/os/windows/fs/{reparse,junction}.rs` behind `#[cfg(windows)]`. A new `StoreAs::Junction(PathBuf)` enum variant is declared unconditionally for exhaustive matching but is only produced on Windows. A new `PathnameEditor::edit_junction` method applies user-specified path transforms to junction targets by delegating to a private helper shared with `edit_symlink` (same semantics: transforms applied, `--strip-components` and leading-slash removal skipped, absolute paths preserved). On Windows extract we create a real junction via `FSCTL_SET_REPARSE_POINT`; on non-Windows we fall back to a symbolic link per the PNA spec's `MAY` clause at `chunk_specifications/index.md:332-336`.

**Tech Stack:** Rust 2024 (MSRV 1.88), existing `windows = "0.62.2"` dependency (already declared in `cli/Cargo.toml:67-73` under `[target.'cfg(windows)'.dependencies]`) with four additional features (`Win32_Foundation`, `Win32_Security`, `Win32_System_Ioctl`, `Win32_System_IO`), existing `fLTP` / `FHED` chunk types (no format changes), existing `--allow-unsafe-links` flag. No changes to `pna/Cargo.toml` or `libpna`.

---

## Scope

**In scope (Phase 1-6):**
- Junction detection on Windows during `create` (NTFS reparse tag = `IO_REPARSE_TAG_MOUNT_POINT`)
- Reparse buffer parsing to extract `SubstituteName`, strip `\??\` prefix → UTF-8 path
- Encoding: `DataKind::HardLink` + `fLTP=Directory` + entry data = external target path (MVP CREATE: always absolute; EXTRACT: accepts both absolute and relative)
- Extract branch: on Windows create junction via `FSCTL_SET_REPARSE_POINT`; on non-Windows create symbolic link
- Extract accepts both absolute and relative stored targets (forward compatibility for a future CREATE-side relative-path optimization, deferred to a separate plan)
- `--allow-unsafe-links` gates extraction of junction entries (junction target is always treated as unsafe)
- Unknown reparse tags (neither mount point nor symlink) during classification → `Ok(None)` with debug log, entry falls through to existing symlink handling
- Regression-free for existing hardlink and symlink tests
- `PathnameEditor::edit_junction` (public) delegates to a private helper that is also used by `edit_symlink`

**Out of scope (deferred):**
- Relative-path optimization on the CREATE side when junction target is inside the archive input set (user directive Q2 — separate future plan)
- Other reparse tags (`IO_REPARSE_TAG_CLOUD_*` OneDrive placeholders, App Execution Aliases, etc.)
- PNA specification document changes (user directive Q6 — implementation defines the encoding; the spec's existing "directory hard link = junction" text stays as-is)
- Windows UNC junctions (the kernel forbids UNC targets for `IO_REPARSE_TAG_MOUNT_POINT`; we do not need to handle them on write, but we must not panic on read)
- `bsdtar compat` subcommand special handling (if junctions flow through its pipeline, they are tolerated — no explicit skip added)

## Prerequisites per Task

1. **Aegis consultation before code changes.** Per project `CLAUDE.md`, every task that touches source files must start by calling `aegis_compile_context` with `target_files` set to the files listed in the task header, `plan` set to the task's action, and `command` set to `"scaffold"` or `"refactor"` as appropriate. Follow returned guidelines. Report `compile_miss` if any guideline was missing.
2. **TDD order.** Each task writes the failing test first, runs it to confirm the failure, implements the minimum change, re-runs, then commits. This order is non-negotiable — `feedback_no_impl_without_plan` applies.
3. **Commit message.** Follow the project convention at `CLAUDE.md`'s emoji table. Do not add `Co-Authored-By` lines (global `CLAUDE.md`).

## Risk Summary

| Risk | Mitigation |
|---|---|
| `DeviceIoControl` unsafe FFI mistakes → UB | Use `windows = "0.62.2"` Rust bindings; wrap all raw pointer math in small helpers; unit-test the reparse-buffer parser with hand-crafted byte slices |
| Junction detection accidentally catches `IO_REPARSE_TAG_SYMLINK` | Strict tag match in `parse_reparse_buffer`; `detect_junction` filters to `ReparsePoint::Junction` only |
| Extract changes break existing hardlink tests | Keep the old HardLink code path untouched; add a separate inner branch gated on `fLTP == Some(Directory)` |
| Symlink fallback on non-Windows creates a user-visible behavior change | Not applicable — no archive created with prior pna versions contains the encoding `HardLink + fLTP=Directory + absolute target` |
| Path traversal via junction target (`..`, `/etc/passwd`) | Junction target is always treated as unsafe; `--allow-unsafe-links` required (same gate as unsafe symlinks) |
| Non-UTF-8 Windows paths lost via `Path::display()` | Use `target.as_os_str().encode_wide()` to build the NT substitute name; never pass through `Display`/`to_string_lossy` |
| `EntryReference` normalizing absolute paths | Use `EntryReference::from_utf8_preserve_root` / `from_path_lossy_preserve_root` (verified at `lib/src/entry/reference.rs:462-476` that `C:\drive\path` and `/abs/path` are preserved verbatim) |

## Testing Matrix (matrix-first per `feedback_test_matrix_first`)

| # | Scenario | Platform | Test kind | Location |
|---|---|---|---|---|
| T1 | Reparse buffer parser — junction SubstituteName extraction and `\??\` strip | Windows | unit | `cli/src/utils/os/windows/fs/reparse.rs` |
| T2 | Reparse buffer parser — truncated buffer yields `InvalidData` error | Windows | unit | same |
| T3 | Reparse buffer parser — unknown tag yields `ReparsePoint::Other(tag)` | Windows | unit | same |
| T4 | `read_reparse_point` on a real junction via `mklink /J` | Windows | unit | same |
| T5 | `create_junction` round-trip with `read_reparse_point` | Windows | unit | `cli/src/utils/os/windows/fs/reparse.rs` |
| T6 | `detect_junction` returns `Some(target)` for junction | Windows | unit | `cli/src/utils/os/windows/fs/junction.rs` |
| T7 | `detect_junction` returns `Ok(None)` for regular directory (via `ERROR_NOT_A_REPARSE_POINT` mapping) | Windows | unit | same |
| T8 | `PathnameEditor::edit_junction` preserves absolute path `C:\abs\target` while applying transforms | cross-platform | unit | `cli/src/command/core/path.rs` |
| T9 | `PathnameEditor::edit_junction` preserves Unix-style absolute `/abs/target` | cross-platform | unit | same |
| T10 | `PathnameEditor::edit_junction` leaves relative path unchanged (no strip, no sanitize) | cross-platform | unit | same |
| T11 | `create_entry` classifies junction and emits HardLink + fLTP=Directory + absolute path | Windows | integration | `cli/tests/cli/junction.rs` |
| T12 | Full round-trip: create archive from junction → extract → junction recreated | Windows | integration | same |
| T13 | Extract HardLink+fLTP=Directory on non-Windows with absolute target → symlink created | Unix | integration | same (using libpna-built fixture) |
| T14 | Extract HardLink+fLTP=Directory with relative target on Windows → junction resolved against extract root | Windows | integration | same |
| T15 | Extract HardLink+fLTP=Directory with relative target on non-Windows → symlink with relative target | Unix | integration | same |
| T16 | Extract without `--allow-unsafe-links` → warn + skip | cross-platform | integration | same |
| T17 | Existing hardlink tests regression-free | cross-platform | existing | `cli/tests/cli/hardlink.rs`, `cli/tests/cli/extract/hardlink.rs` |
| T18 | Existing symlink tests regression-free | cross-platform | existing | `cli/tests/cli/extract/symlink*.rs` |

## File Structure

### New files

| Path | Responsibility |
|---|---|
| `cli/src/utils/os/windows/fs/mod.rs` | Module shim declaring `reparse` and `junction` submodules. |
| `cli/src/utils/os/windows/fs/reparse.rs` | `#[cfg(windows)]` reparse-point FFI primitives: `ReparsePoint` enum, `parse_reparse_buffer` (private parser), `read_reparse_point(path) -> ReparsePoint`, `create_junction(link, target) -> io::Result<()>`. |
| `cli/src/utils/os/windows/fs/junction.rs` | `#[cfg(windows)]` high-level wrapper: `detect_junction(path) -> io::Result<Option<PathBuf>>`. Maps `ERROR_NOT_A_REPARSE_POINT` (raw OS error 4390) to `Ok(None)`. |
| `cli/tests/cli/junction.rs` | Integration tests T11–T16. Windows-gated round-trip tests use `#[cfg(windows)]`; cross-platform extract tests use a libpna-constructed fixture. |

### Modified files

| Path | Change |
|---|---|
| `cli/Cargo.toml` | Extend the existing `windows` dependency `features` array with `Win32_Foundation`, `Win32_Security`, `Win32_System_Ioctl`, `Win32_System_IO`. |
| `cli/src/utils/os/windows/mod.rs` | Declare `pub mod fs;`. |
| `cli/src/command/core/path.rs` | Extract a private helper `fn transform_link_target_preserving_root(&self, target: &Path) -> EntryReference` from the body of `edit_symlink`. Change `edit_symlink` to delegate to it. Add `pub(crate) fn edit_junction(&self, target: &Path) -> EntryReference` that also delegates. |
| `cli/src/command/core.rs` | Add `StoreAs::Junction(PathBuf)` variant (unconditional, dead on non-Windows). Insert junction detection before the existing symlink classification. Add a `create_entry` arm for `StoreAs::Junction`. Add `classify_junction` helper with `#[cfg(windows)]` / `#[cfg(not(windows))]` arms. |
| `cli/src/command/extract.rs` | Inside `DataKind::HardLink` arm, branch on `item.metadata().link_target_type()`; when `Some(Directory)`, call `pathname_editor.edit_junction(target)`, resolve relative targets against `out_dir + link parent`, require `--allow-unsafe-links`, then call the new `create_junction_or_fallback` helper (Windows: `cli::utils::os::windows::fs::reparse::create_junction`; non-Windows: `utils::fs::symlink` with original stored string). |
| `cli/tests/cli/main.rs` (or equivalent test-module registrar) | Register `mod junction;`. |

### Files **not** modified

- `lib/` — `libpna` already round-trips HardLink + fLTP=Directory (verified by existing tests `builder_hardlink_with_link_target_type_directory` at `lib/src/entry/builder.rs:783`, `:798`).
- `pna/` — platform-neutral; no Windows dependency is ever introduced here.
- PNA specification repository — no changes per Q6.

---

## Task List

### Phase 1: CLI Windows reparse-point primitives

Goal: expose a minimal, testable set of wrappers over NTFS reparse points inside the CLI crate. Windows-only. No PNA logic here.

#### Task 1.1: Create module skeletons, Cargo feature bump, and reparse-buffer parser

**Files:**
- Create: `cli/src/utils/os/windows/fs/mod.rs`
- Create: `cli/src/utils/os/windows/fs/reparse.rs`
- Modify: `cli/src/utils/os/windows/mod.rs` (add `pub mod fs;`)
- Modify: `cli/Cargo.toml` (extend `windows` features)

- [ ] **Step 1: Call `aegis_compile_context`**

Call `aegis_compile_context` with:
- `target_files: ["cli/src/utils/os/windows/fs/mod.rs", "cli/src/utils/os/windows/fs/reparse.rs", "cli/src/utils/os/windows/mod.rs", "cli/Cargo.toml"]`
- `plan: "Add Windows reparse-point primitives module in cli with a private parse_reparse_buffer and ReparsePoint enum"`
- `command: "scaffold"`

Follow any guidelines returned.

- [ ] **Step 2: Extend `windows` dependency features in `cli/Cargo.toml`**

Current block at `cli/Cargo.toml:67-73`:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.62.2", features = [
  "Win32_Storage_FileSystem",
  "Win32_Security_Authorization",
  "Win32_System_WindowsProgramming",
  "Win32_System_Threading",
] }
```

Add four features (keep them sorted alphabetically to stabilize diffs):

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.62.2", features = [
  "Win32_Foundation",
  "Win32_Security",
  "Win32_Security_Authorization",
  "Win32_Storage_FileSystem",
  "Win32_System_Ioctl",
  "Win32_System_IO",
  "Win32_System_Threading",
  "Win32_System_WindowsProgramming",
] }
```

- [ ] **Step 3: Create the module skeletons with only the parser tests (no parser implementation yet)**

Create `cli/src/utils/os/windows/fs/mod.rs`:

```rust
pub mod junction;
pub mod reparse;
```

Create `cli/src/utils/os/windows/fs/reparse.rs`:

```rust
//! Windows NTFS reparse-point primitives.
//!
//! This module is Windows-only. All items are behind `#[cfg(windows)]` at the
//! module declaration site (`cli/src/utils/os/windows/mod.rs` is itself
//! already Windows-gated by the parent `os::windows` module).

use std::{io, path::PathBuf};

/// Parsed contents of a reparse point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReparsePoint {
    /// `IO_REPARSE_TAG_MOUNT_POINT` (junction). Target is always absolute as
    /// stored in the reparse buffer after the `\??\` NT-object prefix is stripped.
    Junction(PathBuf),
    /// `IO_REPARSE_TAG_SYMLINK` (file or directory symbolic link). `is_relative`
    /// reflects the `SYMLINK_FLAG_RELATIVE` bit in the reparse header.
    Symlink { target: PathBuf, is_relative: bool },
    /// Any other reparse tag we do not handle (e.g. cloud placeholders,
    /// App Execution Aliases, dedup reparse points).
    Other(u32),
}

/// Parse a raw reparse-point buffer (the payload of `FSCTL_GET_REPARSE_POINT`).
///
/// Errors when the buffer is truncated or contains malformed UTF-16.
pub(crate) fn parse_reparse_buffer(buf: &[u8]) -> io::Result<ReparsePoint> {
    todo!("implemented in Step 5")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a REPARSE_DATA_BUFFER for a junction pointing at `C:\target`.
    fn sample_junction_buffer() -> Vec<u8> {
        // MountPointReparseBuffer layout:
        //   ReparseTag          u32 = 0xA000_0003
        //   ReparseDataLength   u16
        //   Reserved            u16 = 0
        //   SubstituteNameOffset u16
        //   SubstituteNameLength u16
        //   PrintNameOffset     u16
        //   PrintNameLength     u16
        //   PathBuffer          [u16 UTF-16 LE, no null terminator]
        let subst: Vec<u8> = "\\??\\C:\\target"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let print: Vec<u8> = "C:\\target"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let mut path_buffer = subst.clone();
        path_buffer.extend(&print);

        let subst_offset: u16 = 0;
        let subst_len: u16 = subst.len() as u16;
        let print_offset: u16 = subst_len;
        let print_len: u16 = print.len() as u16;

        let mut buf = Vec::new();
        buf.extend(&0xA000_0003u32.to_le_bytes()); // IO_REPARSE_TAG_MOUNT_POINT
        let data_len: u16 = (8 + path_buffer.len()) as u16;
        buf.extend(&data_len.to_le_bytes());
        buf.extend(&0u16.to_le_bytes()); // Reserved
        buf.extend(&subst_offset.to_le_bytes());
        buf.extend(&subst_len.to_le_bytes());
        buf.extend(&print_offset.to_le_bytes());
        buf.extend(&print_len.to_le_bytes());
        buf.extend(&path_buffer);
        buf
    }

    #[test]
    fn parses_junction_and_strips_nt_prefix() {
        let buf = sample_junction_buffer();
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(parsed, ReparsePoint::Junction(PathBuf::from(r"C:\target")));
    }

    #[test]
    fn truncated_buffer_errors() {
        let short = vec![0u8; 4];
        let err = parse_reparse_buffer(&short).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn unknown_tag_is_reported_as_other() {
        // IO_REPARSE_TAG_APPEXECLINK = 0x8000_0023
        let mut buf = vec![0u8; 8];
        buf[0..4].copy_from_slice(&0x8000_0023u32.to_le_bytes());
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(parsed, ReparsePoint::Other(0x8000_0023));
    }
}
```

Modify `cli/src/utils/os/windows/mod.rs` to add `pub mod fs;` near the other `pub mod` declarations (follow existing style).

- [ ] **Step 4: Run the tests to confirm failure**

Run: `cargo test -p portable-network-archive --lib utils::os::windows::fs::reparse::tests --target x86_64-pc-windows-msvc` (on Windows CI; locally this will be a `cargo check --target x86_64-pc-windows-msvc` because `windows` requires the msvc target).

Expected: the first test panics with `not yet implemented` (from `todo!()`), the other two also panic for the same reason.

On a non-Windows developer machine, verify the workspace still compiles cross-platform:

```
cargo check --workspace
```

Expected: no errors. The new module is entirely under `#[cfg(windows)]` so non-Windows builds are unaffected.

- [ ] **Step 5: Implement `parse_reparse_buffer`**

Replace `todo!(...)` with:

```rust
pub(crate) fn parse_reparse_buffer(buf: &[u8]) -> io::Result<ReparsePoint> {
    const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
    const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
    const HEADER_LEN: usize = 8; // ReparseTag(4) + DataLength(2) + Reserved(2)
    const MP_PATHBUF_OFFSET: usize = HEADER_LEN + 8; // + 4 u16 offsets/lens
    const SYMLINK_PATHBUF_OFFSET: usize = HEADER_LEN + 12; // + 4 u16 offsets/lens + Flags(u32)

    if buf.len() < HEADER_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "reparse buffer too short for header",
        ));
    }
    let tag = u32::from_le_bytes(buf[0..4].try_into().unwrap());

    let read_u16 = |offset: usize| -> io::Result<u16> {
        buf.get(offset..offset + 2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "reparse buffer truncated")
            })
    };

    let extract_utf16 =
        |offset: usize, len: u16, pathbuf_base: usize| -> io::Result<String> {
            let start = pathbuf_base + offset as usize;
            let end = start + len as usize;
            let slice = buf.get(start..end).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "reparse path buffer out of range")
            })?;
            if slice.len() % 2 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "odd-length utf16 in reparse path",
                ));
            }
            let utf16: Vec<u16> = slice
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16(&utf16).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid utf16 in reparse path")
            })
        };

    match tag {
        IO_REPARSE_TAG_MOUNT_POINT => {
            let subst_offset = read_u16(HEADER_LEN)?;
            let subst_len = read_u16(HEADER_LEN + 2)?;
            let subst = extract_utf16(subst_offset as usize, subst_len, MP_PATHBUF_OFFSET)?;
            let stripped = subst.strip_prefix(r"\??\").unwrap_or(&subst).to_string();
            Ok(ReparsePoint::Junction(PathBuf::from(stripped)))
        }
        IO_REPARSE_TAG_SYMLINK => {
            if buf.len() < HEADER_LEN + 12 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "reparse buffer too short for symlink flags",
                ));
            }
            let subst_offset = read_u16(HEADER_LEN)?;
            let subst_len = read_u16(HEADER_LEN + 2)?;
            let flags = u32::from_le_bytes(buf[HEADER_LEN + 8..HEADER_LEN + 12].try_into().unwrap());
            let is_relative = flags & 0x1 != 0; // SYMLINK_FLAG_RELATIVE
            let subst = extract_utf16(subst_offset as usize, subst_len, SYMLINK_PATHBUF_OFFSET)?;
            let stripped = if is_relative {
                subst
            } else {
                subst.strip_prefix(r"\??\").unwrap_or(&subst).to_string()
            };
            Ok(ReparsePoint::Symlink {
                target: PathBuf::from(stripped),
                is_relative,
            })
        }
        other => Ok(ReparsePoint::Other(other)),
    }
}
```

- [ ] **Step 6: Run tests to confirm they pass**

Run: `cargo test -p portable-network-archive --lib utils::os::windows::fs::reparse::tests --target x86_64-pc-windows-msvc`.
Expected: all three tests PASS.

- [ ] **Step 7: Commit**

```bash
git add cli/Cargo.toml cli/src/utils/os/windows/mod.rs cli/src/utils/os/windows/fs/mod.rs cli/src/utils/os/windows/fs/reparse.rs
git commit -m ":sparkles: Add Windows reparse buffer parser in CLI"
```

---

#### Task 1.2: `read_reparse_point(path)` — DeviceIoControl wrapper

**Files:**
- Modify: `cli/src/utils/os/windows/fs/reparse.rs`

- [ ] **Step 1: Call `aegis_compile_context`**

`target_files: ["cli/src/utils/os/windows/fs/reparse.rs"]`, `plan: "Add read_reparse_point using DeviceIoControl + FSCTL_GET_REPARSE_POINT"`, `command: "scaffold"`.

- [ ] **Step 2: Write a failing Windows-gated integration-style test**

Append to the existing `tests` module:

```rust
#[test]
fn read_reparse_point_on_junction() -> io::Result<()> {
    use std::process::Command;
    let tmp = tempfile::tempdir()?;
    let target = tmp.path().join("target");
    std::fs::create_dir(&target)?;
    let link = tmp.path().join("link");
    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .status()?;
    assert!(status.success(), "mklink /J failed");

    let rp = super::read_reparse_point(&link)?;
    match rp {
        ReparsePoint::Junction(t) => {
            assert!(
                t.as_os_str().to_string_lossy().ends_with("target"),
                "unexpected junction target {t:?}"
            );
        }
        other => panic!("expected Junction, got {other:?}"),
    }
    Ok(())
}
```

`tempfile` is already in `cli/Cargo.toml` `[dev-dependencies]` (verify with `grep '^tempfile' cli/Cargo.toml`; if missing, add `tempfile = "3"`).

- [ ] **Step 3: Run to confirm failure**

Run: `cargo test -p portable-network-archive --lib utils::os::windows::fs::reparse::tests::read_reparse_point_on_junction --target x86_64-pc-windows-msvc`.
Expected: FAIL with `cannot find function read_reparse_point in module super`.

- [ ] **Step 4: Implement `read_reparse_point`**

Add to `cli/src/utils/os/windows/fs/reparse.rs` (below `parse_reparse_buffer`):

```rust
use std::{os::windows::ffi::OsStrExt, path::Path};

use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, GENERIC_READ, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{Ioctl::FSCTL_GET_REPARSE_POINT, IO::DeviceIoControl},
    },
};

/// Read the reparse data at `path`. Returns `ReparsePoint::Other(tag)` for tags
/// we do not handle.
///
/// Returns an `io::Error` whose raw OS error is `ERROR_NOT_A_REPARSE_POINT`
/// (4390) when `path` is not a reparse point. Callers who want to treat that
/// condition as "not a junction" should inspect `err.raw_os_error()`.
pub fn read_reparse_point(path: &Path) -> io::Result<ReparsePoint> {
    const MAXIMUM_REPARSE_DATA_BUFFER_SIZE: usize = 16 * 1024;

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);

    let handle: HANDLE = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|e| io::Error::from_raw_os_error(e.code().0))?;

    let mut buf = vec![0u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE];
    let mut bytes_returned: u32 = 0;

    let ioctl_result = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_GET_REPARSE_POINT,
            None,
            0,
            Some(buf.as_mut_ptr().cast()),
            buf.len() as u32,
            Some(&mut bytes_returned),
            None,
        )
    };
    // Always attempt to close the handle; do not let a close failure mask an
    // earlier DeviceIoControl error.
    let close_result = unsafe { CloseHandle(handle) };
    ioctl_result.map_err(|e| io::Error::from_raw_os_error(e.code().0))?;
    let _ = close_result;

    buf.truncate(bytes_returned as usize);
    parse_reparse_buffer(&buf)
}
```

- [ ] **Step 5: Run test**

Run: `cargo test -p portable-network-archive --lib utils::os::windows::fs::reparse::tests::read_reparse_point_on_junction --target x86_64-pc-windows-msvc`.
Expected: PASS on Windows CI.

- [ ] **Step 6: Commit**

```bash
git add cli/src/utils/os/windows/fs/reparse.rs
git commit -m ":sparkles: Read NTFS reparse points via DeviceIoControl"
```

---

#### Task 1.3: `create_junction(link, target)` — FSCTL_SET_REPARSE_POINT

**Files:**
- Modify: `cli/src/utils/os/windows/fs/reparse.rs`

- [ ] **Step 1: Call `aegis_compile_context`**

`target_files: ["cli/src/utils/os/windows/fs/reparse.rs"]`, `plan: "Add create_junction using FSCTL_SET_REPARSE_POINT"`, `command: "scaffold"`.

- [ ] **Step 2: Write the failing test**

```rust
#[test]
fn create_junction_round_trip() -> io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let target = tmp.path().join("target");
    std::fs::create_dir(&target)?;
    let link = tmp.path().join("junction");
    super::create_junction(&link, &target)?;

    let rp = super::read_reparse_point(&link)?;
    match rp {
        ReparsePoint::Junction(t) => assert!(
            t.as_os_str().to_string_lossy().ends_with("target"),
            "unexpected junction target {t:?}"
        ),
        other => panic!("expected Junction, got {other:?}"),
    }
    Ok(())
}
```

- [ ] **Step 3: Run to confirm failure**

Run: `cargo test -p portable-network-archive --lib utils::os::windows::fs::reparse::tests::create_junction_round_trip --target x86_64-pc-windows-msvc`.
Expected: FAIL with `cannot find function create_junction in module super`.

- [ ] **Step 4: Implement `create_junction`**

Add to `cli/src/utils/os/windows/fs/reparse.rs`:

```rust
use windows::Win32::{
    Foundation::GENERIC_WRITE,
    System::Ioctl::FSCTL_SET_REPARSE_POINT,
};

/// Create a junction at `link` pointing to the absolute `target`.
///
/// `link` must not already exist; `target` must be an absolute path to a
/// directory.
///
/// Path bytes are preserved exactly via `OsStr::encode_wide`; `Path::display`
/// and `to_string_lossy` are deliberately not used because they would drop
/// unpaired UTF-16 surrogates.
pub fn create_junction(link: &Path, target: &Path) -> io::Result<()> {
    if !target.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "junction target must be absolute",
        ));
    }
    std::fs::create_dir(link)?;

    // Open the freshly-created directory with read+write and reparse semantics.
    let mut link_wide: Vec<u16> = link.as_os_str().encode_wide().collect();
    link_wide.push(0);
    let handle: HANDLE = unsafe {
        CreateFileW(
            PCWSTR(link_wide.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|e| {
        let err = io::Error::from_raw_os_error(e.code().0);
        let _ = std::fs::remove_dir(link);
        err
    })?;

    // SubstituteName = \??\ + target (as raw UTF-16, no lossy conversion)
    let mut subst_wide: Vec<u16> = r"\??\".encode_utf16().collect();
    subst_wide.extend(target.as_os_str().encode_wide());
    let print_wide: Vec<u16> = target.as_os_str().encode_wide().collect();

    let subst_bytes_len = (subst_wide.len() * 2) as u16;
    let print_bytes_len = (print_wide.len() * 2) as u16;

    let mut buf = Vec::<u8>::new();
    buf.extend(&0xA000_0003u32.to_le_bytes()); // ReparseTag = IO_REPARSE_TAG_MOUNT_POINT
    let data_len: u16 = 8 + subst_bytes_len + print_bytes_len;
    buf.extend(&data_len.to_le_bytes());       // ReparseDataLength
    buf.extend(&0u16.to_le_bytes());           // Reserved
    buf.extend(&0u16.to_le_bytes());           // SubstituteNameOffset
    buf.extend(&subst_bytes_len.to_le_bytes()); // SubstituteNameLength
    buf.extend(&subst_bytes_len.to_le_bytes()); // PrintNameOffset (right after subst)
    buf.extend(&print_bytes_len.to_le_bytes()); // PrintNameLength
    for u in &subst_wide { buf.extend(&u.to_le_bytes()); }
    for u in &print_wide { buf.extend(&u.to_le_bytes()); }

    let mut bytes_returned = 0u32;
    let ioctl_result = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_SET_REPARSE_POINT,
            Some(buf.as_ptr().cast()),
            buf.len() as u32,
            None,
            0,
            Some(&mut bytes_returned),
            None,
        )
    };
    let close_result = unsafe { CloseHandle(handle) };

    if let Err(e) = ioctl_result {
        let err = io::Error::from_raw_os_error(e.code().0);
        let _ = std::fs::remove_dir(link);
        return Err(err);
    }
    let _ = close_result;
    Ok(())
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p portable-network-archive --lib utils::os::windows::fs::reparse::tests --target x86_64-pc-windows-msvc`.
Expected: all reparse tests PASS.

- [ ] **Step 6: Commit**

```bash
git add cli/src/utils/os/windows/fs/reparse.rs
git commit -m ":sparkles: Create NTFS junctions via FSCTL_SET_REPARSE_POINT"
```

---

#### Task 1.4: `detect_junction(path)` wrapper with `ERROR_NOT_A_REPARSE_POINT` mapping

**Files:**
- Create: `cli/src/utils/os/windows/fs/junction.rs`

- [ ] **Step 1: Call `aegis_compile_context`**

`target_files: ["cli/src/utils/os/windows/fs/junction.rs"]`, `plan: "Add detect_junction helper that maps ERROR_NOT_A_REPARSE_POINT to Ok(None)"`, `command: "scaffold"`.

- [ ] **Step 2: Write the test and skeleton**

Create `cli/src/utils/os/windows/fs/junction.rs`:

```rust
//! Windows junction detection for the CLI create path.
//!
//! This module is only compiled on Windows; other platforms get a shim arm
//! at the call site in `cli/src/command/core.rs`.

use std::{io, path::{Path, PathBuf}};

use crate::utils::os::windows::fs::reparse::{read_reparse_point, ReparsePoint};

/// Win32 error code returned by `FSCTL_GET_REPARSE_POINT` when the target is
/// not a reparse point. See <https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--4000-5999->.
const ERROR_NOT_A_REPARSE_POINT: i32 = 4390;

/// If `path` is a junction, returns its absolute target; otherwise `Ok(None)`.
///
/// Returns `Ok(None)` for:
/// - Non-reparse paths (mapped from `ERROR_NOT_A_REPARSE_POINT`)
/// - Regular symlinks (`ReparsePoint::Symlink`)
/// - Unknown reparse tags (`ReparsePoint::Other`)
///
/// Propagates other I/O errors (permission denied, invalid handle, etc.) to
/// the caller so they are surfaced as create-time errors.
pub fn detect_junction(path: &Path) -> io::Result<Option<PathBuf>> {
    match read_reparse_point(path) {
        Ok(ReparsePoint::Junction(t)) => Ok(Some(t)),
        Ok(_) => Ok(None),
        Err(e) if e.raw_os_error() == Some(ERROR_NOT_A_REPARSE_POINT) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_directory_is_not_junction() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        assert_eq!(detect_junction(tmp.path())?, None);
        Ok(())
    }

    #[test]
    fn real_junction_detected() -> io::Result<()> {
        use std::process::Command;
        let tmp = tempfile::tempdir()?;
        let target = tmp.path().join("target");
        std::fs::create_dir(&target)?;
        let link = tmp.path().join("link");
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .status()?;
        assert!(status.success(), "mklink /J failed");

        let t = detect_junction(&link)?.expect("junction should be detected");
        assert!(
            t.as_os_str().to_string_lossy().ends_with("target"),
            "unexpected junction target {t:?}"
        );
        Ok(())
    }
}
```

- [ ] **Step 3: Run the test**

Run: `cargo test -p portable-network-archive --lib utils::os::windows::fs::junction::tests --target x86_64-pc-windows-msvc`.
Expected: both tests PASS on Windows CI.

Also run on the host (macOS/Linux) to confirm cross-platform build is unaffected:

```
cargo check --workspace
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add cli/src/utils/os/windows/fs/junction.rs
git commit -m ":sparkles: Add detect_junction helper for CLI"
```

---

### Phase 2: `PathnameEditor::edit_junction`

#### Task 2.1: Extract shared helper, add `edit_junction` delegating to it

**Files:**
- Modify: `cli/src/command/core/path.rs`

- [ ] **Step 1: Call `aegis_compile_context`**

`target_files: ["cli/src/command/core/path.rs"]`, `plan: "Extract a private helper from edit_symlink and add edit_junction that delegates to it"`, `command: "refactor"`.

- [ ] **Step 2: Write failing unit tests for `edit_junction`**

Append to the existing `tests` module in `cli/src/command/core/path.rs`:

```rust
#[test]
fn edit_junction_preserves_unix_absolute() {
    let editor = PathnameEditor::new(None, None, false, false);
    let out = editor.edit_junction(Path::new("/abs/target"));
    assert_eq!(out.as_str(), "/abs/target");
}

#[test]
fn edit_junction_preserves_windows_absolute() {
    let editor = PathnameEditor::new(None, None, false, false);
    let out = editor.edit_junction(Path::new("C:\\abs\\target"));
    assert_eq!(out.as_str(), "C:\\abs\\target");
}

#[test]
fn edit_junction_preserves_relative_unchanged() {
    let editor = PathnameEditor::new(None, None, false, false);
    let out = editor.edit_junction(Path::new("rel/target"));
    assert_eq!(out.as_str(), "rel/target");
}

#[test]
fn edit_junction_does_not_apply_strip_components() {
    let editor = PathnameEditor::new(Some(1), None, false, false);
    let out = editor.edit_junction(Path::new("/abs/target"));
    // strip_components does NOT apply to junction targets, matching symlink semantics.
    assert_eq!(out.as_str(), "/abs/target");
}
```

- [ ] **Step 3: Run to confirm failure**

Run: `cargo test -p portable-network-archive --lib command::core::path::tests::edit_junction_`.
Expected: FAIL with `no method named 'edit_junction' found for reference '&PathnameEditor'`.

- [ ] **Step 4: Extract shared helper and add `edit_junction`**

Current `edit_symlink` (`cli/src/command/core/path.rs:84-95`):

```rust
pub(crate) fn edit_symlink(&self, target: &Path) -> EntryReference {
    let transformed: Cow<'_, Path> = if let Some(t) = &self.transformers {
        Cow::Owned(PathBuf::from(t.apply(
            target.to_string_lossy(),
            true,
            false,
        )))
    } else {
        Cow::Borrowed(target)
    };
    EntryReference::from_path_lossy_preserve_root(&transformed)
}
```

Replace with:

```rust
/// Apply user-specified substitutions to a link target while preserving
/// absolute path components and skipping `--strip-components`, matching
/// bsdtar symlink semantics. Shared between [`edit_symlink`](Self::edit_symlink)
/// and [`edit_junction`](Self::edit_junction).
fn transform_link_target_preserving_root(&self, target: &Path) -> EntryReference {
    let transformed: Cow<'_, Path> = if let Some(t) = &self.transformers {
        Cow::Owned(PathBuf::from(t.apply(
            target.to_string_lossy(),
            true,
            false,
        )))
    } else {
        Cow::Borrowed(target)
    };
    EntryReference::from_path_lossy_preserve_root(&transformed)
}

/// Edit a symlink target path.
///
/// Only user-specified substitutions (`-s`) are applied.
/// Leading `/` and `--strip-components` are NOT applied, matching bsdtar.
pub(crate) fn edit_symlink(&self, target: &Path) -> EntryReference {
    self.transform_link_target_preserving_root(target)
}

/// Edit a Windows-junction target path.
///
/// Semantically identical to [`edit_symlink`](Self::edit_symlink) for the
/// moment (same substitution treatment, same preservation of absolute path
/// components). A separate public method is introduced so that any future
/// divergence between symlink-target and junction-target handling can be
/// added without touching every call site.
pub(crate) fn edit_junction(&self, target: &Path) -> EntryReference {
    self.transform_link_target_preserving_root(target)
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p portable-network-archive --lib command::core::path`.
Expected: existing `edit_symlink` tests + new `edit_junction` tests all PASS.

- [ ] **Step 6: Commit**

```bash
git add cli/src/command/core/path.rs
git commit -m ":sparkles: Add PathnameEditor::edit_junction delegating to shared helper"
```

---

### Phase 3: CLI create path

#### Task 3.1: Add `StoreAs::Junction(PathBuf)` variant and wire classification

**Files:**
- Modify: `cli/src/command/core.rs`

- [ ] **Step 1: Call `aegis_compile_context`**

`target_files: ["cli/src/command/core.rs"]`, `plan: "Add StoreAs::Junction variant (unconditional), wire junction classification before symlink classification on Windows, and add create_entry arm"`, `command: "refactor"`.

- [ ] **Step 2: Write the failing integration test**

Create `cli/tests/cli/junction.rs`:

```rust
//! Integration tests for Windows junction support.

use std::{path::PathBuf, process::Command};

use pna::{prelude::*, Archive, DataKind, EntryBuilder, EntryName, EntryReference,
           LinkTargetType, ReadOptions, WriteOptions};

#[cfg(windows)]
fn mklink_junction(link: &std::path::Path, target: &std::path::Path) {
    let status = Command::new("cmd")
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
    let status = Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
        .arg(&archive_path)
        .args(["link_dir", "target_dir"])
        .status()
        .unwrap();
    assert!(status.success());

    let bytes = std::fs::read(&archive_path).unwrap();
    let archive = Archive::read_header(&bytes[..]).unwrap();
    let mut saw_junction_entry = false;
    for entry in archive.entries_slice() {
        let entry = entry.unwrap();
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
```

Register the test module. Find the existing test registrar (likely `cli/tests/cli/main.rs` or the crate's `tests/cli.rs` root). Add `mod junction;` next to the other `mod <name>;` lines.

- [ ] **Step 3: Run the test to confirm failure**

Run: `cargo test -p portable-network-archive --test cli create_records_junction_as_hardlink_directory --target x86_64-pc-windows-msvc`.
Expected: FAIL — current classification records the junction as a broken Symlink (or fails walk) because there is no `StoreAs::Junction` yet.

- [ ] **Step 4: Add `StoreAs::Junction` variant**

Edit `cli/src/command/core.rs` at the `StoreAs` enum (lines 379-384 today):

```rust
#[derive(Clone, Debug)]
pub(crate) enum StoreAs {
    File,
    Dir,
    Symlink(LinkTargetType),
    Hardlink(PathBuf),
    /// Windows NTFS junction. The inner `PathBuf` is the **external** target
    /// path (typically absolute). This variant is only produced on Windows
    /// but is declared unconditionally so that `match` arms remain exhaustive
    /// on every platform.
    Junction(PathBuf),
}
```

- [ ] **Step 5: Add `classify_junction` helper**

Near the bottom of `core.rs`, alongside `detect_symlink_target_type`, add:

```rust
/// Returns the junction target if `path` is a Windows junction, or `None` for
/// any non-junction path (including regular directories, symlinks, and
/// unknown reparse tags). Errors are swallowed to a debug log so classification
/// still falls through to the existing symlink handler.
#[cfg(windows)]
fn classify_junction(path: &Path) -> io::Result<Option<PathBuf>> {
    match crate::utils::os::windows::fs::junction::detect_junction(path) {
        Ok(v) => Ok(v),
        Err(e) => {
            log::debug!("Failed to inspect reparse point {}: {}", path.display(), e);
            Ok(None)
        }
    }
}

#[cfg(not(windows))]
fn classify_junction(_path: &Path) -> io::Result<Option<PathBuf>> {
    Ok(None)
}
```

- [ ] **Step 6: Detect junction before symlink classification**

In `core.rs` around lines 738-741, replace:

```rust
let store = if is_symlink {
    let meta = fs::symlink_metadata(path)?;
    let link_target_type = detect_symlink_target_type(path, &meta)?;
    Some((StoreAs::Symlink(link_target_type), meta))
```

with:

```rust
let store = if is_symlink {
    let meta = fs::symlink_metadata(path)?;
    if let Some(target) = classify_junction(path)? {
        Some((StoreAs::Junction(target), meta))
    } else {
        let link_target_type = detect_symlink_target_type(path, &meta)?;
        Some((StoreAs::Symlink(link_target_type), meta))
    }
```

- [ ] **Step 7: Add `create_entry` arm for `StoreAs::Junction`**

Find the existing `match store_as` around line 914 (after the `Hardlink` arm). Insert:

```rust
StoreAs::Junction(target) => {
    // Junction target is an external absolute path; it is NOT an archive
    // entry name. The libpna HardLink + fLTP=Directory encoding round-trips
    // this path verbatim.
    let reference = EntryReference::from_path_lossy_preserve_root(target.as_path());
    let mut entry = EntryBuilder::new_hard_link(entry_name.clone(), reference)?;
    entry.link_target_type(LinkTargetType::Directory);
    apply_metadata(entry, path, keep_options, metadata)?.build()
}
```

Note: `apply_metadata` is the existing helper that wires timestamps, ownership, xattrs, etc. onto the builder. Verify the exact call convention against the adjacent `Hardlink` arm and match it.

Verify `EntryReference::from_path_lossy_preserve_root` exists (it does — see `lib/src/entry/reference.rs:120`). If for any reason a stricter constructor is needed, use `from_utf8_preserve_root(&target.to_string_lossy())` instead — both produce the same result for valid UTF-8 paths.

- [ ] **Step 8: Update any other `match StoreAs` sites**

Run `rg 'StoreAs::' cli/src` to enumerate. Verification at time of planning showed the only non-enum site is `cli/src/command/core.rs` itself. Confirm that the walker/dispatch match in `collect_items_from_paths` (and any peer sites found) either handles `Junction` or delegates to a fall-through that does not panic.

If a match over `StoreAs` is expected to be closed but does not yet handle `Junction`, add an arm that treats junction identically to `Hardlink` for display/counting purposes (since the on-disk encoding is the same).

- [ ] **Step 9: Run tests**

Run: `cargo test -p portable-network-archive --test cli create_records_junction_as_hardlink_directory --target x86_64-pc-windows-msvc`.
Expected: PASS on Windows CI.

Run: `cargo check --workspace` on the host to confirm non-Windows builds unaffected.

Run: `cargo test --workspace --all-features` to catch regressions in T17/T18.

- [ ] **Step 10: Commit**

```bash
git add cli/src/command/core.rs cli/tests/cli/junction.rs cli/tests/cli/main.rs
git commit -m ":sparkles: Detect and archive Windows junctions as HardLink entries"
```

---

### Phase 4: CLI extract path

#### Task 4.1: Branch `DataKind::HardLink` on `fLTP=Directory`, use `edit_junction`, handle both absolute and relative stored targets

**Files:**
- Modify: `cli/src/command/extract.rs`

- [ ] **Step 1: Call `aegis_compile_context`**

`target_files: ["cli/src/command/extract.rs"]`, `plan: "Branch DataKind::HardLink on fLTP=Directory to create a junction (Windows) or symlink fallback (non-Windows); accept both absolute and relative stored targets; apply edit_junction; require --allow-unsafe-links"`, `command: "refactor"`.

- [ ] **Step 2: Write a failing cross-platform fixture test**

Append to `cli/tests/cli/junction.rs` (outside the `#[cfg(windows)]` block — these tests build fixtures via libpna and run on any platform):

```rust
/// Build an in-memory archive containing one HardLink+fLTP=Directory entry
/// whose target is the supplied path string (interpreted verbatim).
fn build_junction_fixture(target: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut archive = Archive::write_header(&mut out).unwrap();
    let name: EntryName = "link_dir".parse().unwrap();
    let reference = EntryReference::from_utf8_preserve_root(target);
    let mut builder = EntryBuilder::new_hard_link(name, reference).unwrap();
    builder.link_target_type(LinkTargetType::Directory);
    let entry = builder.build().unwrap();
    archive
        .add_entry(entry, WriteOptions::builder().build())
        .unwrap();
    archive.finalize().unwrap();
    out
}

/// Precondition: archive with a HardLink+fLTP=Directory entry pointing at a
/// well-known absolute path.
/// Action: extract without `--allow-unsafe-links`.
/// Expectation: the entry is skipped with a warning and no link is created.
#[test]
fn extract_junction_without_allow_unsafe_links_skips() {
    let tmp = tempfile::tempdir().unwrap();
    let archive_path = tmp.path().join("fixture.pna");
    std::fs::write(&archive_path, build_junction_fixture("/any/absolute/path")).unwrap();

    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pna"))
        .args(["extract", "-f"])
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!out_dir.join("link_dir").exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsafe") || stderr.contains("allow-unsafe-links"),
        "expected skip warning, got: {stderr}"
    );
}
```

- [ ] **Step 3: Run to confirm failure**

Run: `cargo test -p portable-network-archive --test cli extract_junction_without_allow_unsafe_links_skips`.
Expected: FAIL — current extract code calls `edit_hardlink` which strips the leading `/`, then `fs::hard_link` fails or creates a regular hardlink (which POSIX rejects on directories).

- [ ] **Step 4: Implement the inner branch**

In `cli/src/command/extract.rs`, locate the `DataKind::HardLink => { ... }` arm (currently lines 1426-1455 inside `extract_link_entry`). Replace it with:

```rust
DataKind::HardLink => {
    let reader = item.reader(ReadOptions::with_password(password))?;
    let original = io::read_to_string(reader)?;
    let is_directory_link = matches!(
        item.metadata().link_target_type(),
        Some(LinkTargetType::Directory)
    );

    if is_directory_link {
        // Encoded junction: apply user transforms but preserve absolute paths
        // and do NOT sanitize (no edit_hardlink).
        let transformed = pathname_editor.edit_junction(Path::new(original.as_str()));
        let target_str = transformed.as_str();

        if !allow_unsafe_links {
            log::warn!(
                "Skipped extracting a junction (HardLink+fLTP=Directory). \
                 Use `--allow-unsafe-links` to create it."
            );
            return Ok(());
        }
        if *safe_writes || remove_existing {
            utils::io::ignore_not_found(utils::fs::remove_path(&path))?;
        }
        create_junction_or_fallback(&path, target_str)?;
    } else {
        // Existing hardlink path — unchanged behavior.
        let Some((original, had_root)) = pathname_editor.edit_hardlink(original.as_ref())
        else {
            log::warn!(
                "Skipped extracting a hard link that pointed at a file which was skipped.: {}",
                original
            );
            return Ok(());
        };
        if had_root && !warned_lead_slash.swap(true, Ordering::Relaxed) {
            eprintln!("bsdtar: Removing leading '/' from member names");
        }
        if !allow_unsafe_links && is_unsafe_link(&original) {
            log::warn!(
                "Skipped extracting a hard link that contains an unsafe link. If you need to extract it, use `--allow-unsafe-links`."
            );
            return Ok(());
        }
        let original = if let Some(out_dir) = out_dir {
            Cow::from(out_dir.join(original))
        } else {
            Cow::from(original.as_path())
        };
        if *safe_writes || remove_existing {
            utils::io::ignore_not_found(utils::fs::remove_path(&path))?;
        }
        fs::hard_link(original, &path)?;
    }
}
```

Add the `create_junction_or_fallback` helper near the bottom of `extract.rs`:

```rust
/// Create the link for a HardLink+fLTP=Directory entry.
///
/// On Windows, builds a real junction. On non-Windows, falls back to a
/// symbolic link per the PNA spec `chunk_specifications/index.md:332-336`
/// MAY clause. Accepts both absolute and relative stored targets:
///
/// - On Windows, relative targets are resolved against `link`'s parent then
///   canonicalized to an absolute path (kernel requires absolute for
///   junctions). If canonicalization fails (e.g. the target does not exist),
///   the join result is passed through; `create_junction` will then fail with
///   a descriptive I/O error.
/// - On non-Windows, the raw stored string is passed to `symlink` verbatim
///   so the resulting symlink is identical to what the archive encoded.
fn create_junction_or_fallback(link: &Path, target: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        let raw = Path::new(target);
        let absolute = if raw.is_absolute() {
            raw.to_path_buf()
        } else {
            let base = link.parent().unwrap_or_else(|| Path::new("."));
            let joined = base.join(raw);
            std::fs::canonicalize(&joined).unwrap_or(joined)
        };
        crate::utils::os::windows::fs::reparse::create_junction(link, &absolute)
    }
    #[cfg(not(windows))]
    {
        log::warn!(
            "Creating symbolic link instead of Windows junction on non-Windows platform: {} -> {}",
            link.display(),
            target
        );
        crate::utils::fs::symlink(target, link)
    }
}
```

Ensure the top of `extract.rs` imports `LinkTargetType` from `pna`. If it is already imported in the module's `use` block (grep confirms it is imported near the top), no change needed.

- [ ] **Step 5: Run tests**

Run: `cargo test -p portable-network-archive --test cli extract_junction_without_allow_unsafe_links_skips`.
Expected: PASS.

Run: `cargo test --workspace --all-features`.
Expected: no regressions in `cli/tests/cli/hardlink.rs` or `extract/hardlink.rs`.

- [ ] **Step 6: Commit**

```bash
git add cli/src/command/extract.rs cli/tests/cli/junction.rs
git commit -m ":sparkles: Extract HardLink+fLTP=Directory as junction or symlink fallback"
```

---

#### Task 4.2: Cross-platform extract round-trip with `--allow-unsafe-links`

**Files:**
- Modify: `cli/tests/cli/junction.rs` (tests only)

- [ ] **Step 1: Write the test**

Append to `cli/tests/cli/junction.rs`:

```rust
/// Precondition: archive with a HardLink+fLTP=Directory entry pointing at an
/// existing absolute path.
/// Action: extract with `--allow-unsafe-links`.
/// Expectation: on Windows a junction is created; on non-Windows a symlink
/// whose target string is exactly the stored absolute path.
#[test]
fn extract_junction_with_allow_unsafe_links_creates_link() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("actual_target");
    std::fs::create_dir(&target).unwrap();
    let archive_path = tmp.path().join("fixture.pna");
    let target_str = target.to_string_lossy().into_owned();
    std::fs::write(&archive_path, build_junction_fixture(&target_str)).unwrap();

    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_pna"))
        .args(["extract", "-f"])
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--allow-unsafe-links")
        .status()
        .unwrap();
    assert!(status.success());

    let link = out_dir.join("link_dir");
    let meta = std::fs::symlink_metadata(&link).unwrap();

    #[cfg(windows)]
    {
        use crate::junction::ReparseHandle as _; // no-op, keep imports consistent
        use portable_network_archive::utils as _; // keep public-only usage
        // Use the public reparse helpers to verify junction flavor.
        // (Cross-crate test access; if not public, fall back to FileType check below.)
        let _ = meta;
        // At minimum, the created link must be a reparse point; extended
        // verification happens in the Windows-only round_trip_junction_via_cli
        // test.
        let ft = std::fs::symlink_metadata(&link).unwrap().file_type();
        use std::os::windows::fs::FileTypeExt;
        assert!(
            ft.is_symlink() || ft.is_symlink_dir() || ft.is_symlink_file(),
            "expected a reparse-point flavored symlink-like file"
        );
    }
    #[cfg(not(windows))]
    {
        assert!(meta.file_type().is_symlink());
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            PathBuf::from(&target_str)
        );
    }
}
```

If the Windows-side `use` aliases above cause compile issues because the referenced types are not public outside the crate, simplify the Windows branch to only assert `meta.file_type().is_symlink()` (or the `_dir`/`_file` variants). The full junction-flavor assertion is already covered by `round_trip_junction_via_cli` in Task 4.3.

- [ ] **Step 2: Run**

Run: `cargo test -p portable-network-archive --test cli extract_junction_with_allow_unsafe_links_creates_link` on the host (macOS/Linux) and on Windows CI.
Expected: PASS on both.

- [ ] **Step 3: Commit**

```bash
git add cli/tests/cli/junction.rs
git commit -m ":white_check_mark: Cover junction extract round-trip across platforms"
```

---

#### Task 4.3: Full round-trip on Windows

**Files:**
- Modify: `cli/tests/cli/junction.rs`

- [ ] **Step 1: Write the test (Windows-only)**

Append to `cli/tests/cli/junction.rs`:

```rust
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
    assert!(Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
        .arg(&archive_path)
        .args(["link_dir", "target_dir"])
        .status()
        .unwrap()
        .success());

    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    assert!(Command::new(env!("CARGO_BIN_EXE_pna"))
        .args(["extract", "-f"])
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--allow-unsafe-links")
        .status()
        .unwrap()
        .success());

    let link = out_dir.join("link_dir");
    let meta = std::fs::symlink_metadata(&link).unwrap();
    use std::os::windows::fs::FileTypeExt;
    assert!(
        meta.file_type().is_symlink() || meta.file_type().is_symlink_dir(),
        "expected a reparse point, got {:?}",
        meta.file_type()
    );

    // Deep-verify the reparse tag via cmd (avoids needing to expose internal
    // helpers across crate boundaries).
    let output = Command::new("cmd")
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
```

- [ ] **Step 2: Run**

Run: `cargo test -p portable-network-archive --test cli round_trip_junction_via_cli --target x86_64-pc-windows-msvc`.
Expected: PASS on Windows CI.

- [ ] **Step 3: Commit**

```bash
git add cli/tests/cli/junction.rs
git commit -m ":white_check_mark: End-to-end junction round trip via CLI"
```

---

### Phase 5: Hardening

#### Task 5.1: Debug-log reparse-inspection failures; keep user-visible behavior unchanged

**Files:**
- Modify: `cli/src/command/core.rs` (if the debug-log wrapper was not already added in Task 3.1 Step 5)

Decision lock-in: **option (A) — debug-level logging only**. Unknown reparse tags and reparse inspection errors must not emit `warn`-level messages because the current behavior for symlinks is not to warn on classification miss; preserving that parity is conservative. A louder signal can be added as a follow-up PR if users need it.

- [ ] **Step 1: Call `aegis_compile_context`**

`target_files: ["cli/src/command/core.rs"]`, `plan: "Ensure classify_junction logs errors at debug level and returns Ok(None) so fall-through remains silent"`, `command: "refactor"`.

- [ ] **Step 2: Verify the helper added in Task 3.1 Step 5 is already correct**

Re-read the `classify_junction` helper introduced in Task 3.1. The `#[cfg(windows)]` arm must be:

```rust
#[cfg(windows)]
fn classify_junction(path: &Path) -> io::Result<Option<PathBuf>> {
    match crate::utils::os::windows::fs::junction::detect_junction(path) {
        Ok(v) => Ok(v),
        Err(e) => {
            log::debug!("Failed to inspect reparse point {}: {}", path.display(), e);
            Ok(None)
        }
    }
}
```

If Task 3.1 produced a version that returns `Err(e)` directly instead of logging and swallowing, update it now.

- [ ] **Step 3: Add a cross-platform unit test for the `Other` reparse tag branch**

Append to the existing `tests` module in `cli/src/utils/os/windows/fs/reparse.rs` (the test can be Windows-only because the module itself is `#[cfg(windows)]`):

```rust
#[test]
fn appexec_tag_is_other() {
    let mut buf = vec![0u8; 8];
    buf[0..4].copy_from_slice(&0x8000_0023u32.to_le_bytes()); // IO_REPARSE_TAG_APPEXECLINK
    let parsed = parse_reparse_buffer(&buf).unwrap();
    assert_eq!(parsed, ReparsePoint::Other(0x8000_0023));
}
```

(If the equivalent assertion was already added in Task 1.1 Step 3 as `unknown_tag_is_reported_as_other`, skip this step to avoid duplication.)

- [ ] **Step 4: Run and commit**

```
cargo test -p portable-network-archive --lib utils::os::windows::fs::reparse::tests --target x86_64-pc-windows-msvc
cargo test -p portable-network-archive --test cli
```

Commit only if files were modified this task:

```bash
git add cli/src/command/core.rs
git commit -m ":sparkles: Log reparse inspection failures at debug level"
```

---

#### Task 5.2: `bsdtar compat` regression check

**Files:**
- Verify only; no code changes are expected

Decision: junctions that reach the bsdtar-compat pipeline are **tolerated** (no explicit skip). The pipeline shares `collect_items` and `PathnameEditor`, so any junction encountered will be encoded identically. Users who want bsdtar-identical output can avoid junctions in their input tree.

- [ ] **Step 1: Regression-scan the bsdtar command family**

Run: `rg -n "StoreAs::" cli/src/command/bsdtar.rs cli/src/command/compat.rs cli/src/command/core/mtree.rs`.
Expected: no `StoreAs::` match that would need a new arm. If any match site turns up, add a `StoreAs::Junction(_)` arm that routes identically to the existing `Hardlink` arm.

Run the bsdtar-compat test batteries if the environment supports them:

```
cargo test -p portable-network-archive --test cli compat
cargo test -p portable-network-archive --test cli bsdtar
```

Expected: no regressions.

- [ ] **Step 2: If any change was required, commit**

```bash
git commit -m ":recycle: Route junction entries through bsdtar compat pipeline"
```

Otherwise skip.

---

#### Task 5.3: Coverage sweep

- [ ] **Step 1: Run coverage on the new files**

If the `cargo llvm-cov` tool is available:

```bash
cargo +nightly llvm-cov --workspace --all-features --html \
  --open --target x86_64-pc-windows-msvc
```

Expect the report to list `cli/src/utils/os/windows/fs/reparse.rs`, `cli/src/utils/os/windows/fs/junction.rs`, the new arms in `core.rs`, the new branch in `extract.rs`, and the new method in `path.rs`. Target ≥80% line coverage on each.

- [ ] **Step 2: Add tests for any non-trivial uncovered path**

If coverage reveals uncovered branches (e.g. error propagation from `DeviceIoControl`), add unit tests; otherwise annotate with `// coverage: Windows-only` comments.

- [ ] **Step 3: Commit if tests added**

```bash
git commit -m ":white_check_mark: Improve coverage of junction paths"
```

---

### Phase 6: Docs and manual verification

#### Task 6.1: Man page / CLI help

**Files:**
- Review: wherever `--allow-unsafe-links` help text is defined (likely `cli/src/cli.rs` or a subcommand arg struct)
- Review: `xtask`-generated docs

- [ ] **Step 1: Grep for the existing help string**

Run: `rg -n "allow-unsafe-links" cli/src`.

- [ ] **Step 2: Update the help text**

Change the help string to something like:

```
Allow extracting symbolic links, hard links, or Windows junctions whose target points outside the extraction root.
```

- [ ] **Step 3: Regenerate docs**

```bash
cargo xtask mangen --output target/man
cargo xtask docgen --output target/doc/pna.md
```

- [ ] **Step 4: Commit**

```bash
git add cli/src/cli.rs
git commit -m ":memo: Document junction handling in --allow-unsafe-links"
```

---

#### Task 6.2: Verification-before-completion

- [ ] **Step 1: Full workspace test**

```
cargo test --workspace --all-features
```

Expected: PASS on the host.

- [ ] **Step 2: Clippy**

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: no warnings.

- [ ] **Step 3: Format**

```
cargo fmt --all -- --check
```

Expected: no diff.

- [ ] **Step 4: Feature-powerset (if time permits)**

```
cargo hack check --locked --feature-powerset --exclude-features wasm
```

Expected: all combinations compile.

- [ ] **Step 5: On a Windows machine, perform a manual smoke test**

```powershell
mkdir C:\tmp\junc-test\real
mklink /J C:\tmp\junc-test\link C:\tmp\junc-test\real
pna create -f C:\tmp\junc-test\out.pna C:\tmp\junc-test
pna extract -f C:\tmp\junc-test\out.pna --out-dir C:\tmp\junc-test-out --allow-unsafe-links
dir /AL C:\tmp\junc-test-out\junc-test
```

Expected: the listing shows `link` as `<JUNCTION>` pointing at `C:\tmp\junc-test\real`.

- [ ] **Step 6: Push branch and stop** (per `feedback_stop_at_push`: no PR creation unless explicitly requested)

```bash
git push -u origin feat/windows-junction-support
```

After push, run `gh run list --branch feat/windows-junction-support --limit 5` once to confirm CI started (per `feedback_check_ci_after_push`). Do NOT open a pull request.

---

## Out-of-Scope Follow-up (placeholder for a separate plan)

**Relative-path optimization in CREATE (user directive Q2):** when a junction's absolute target is within the archive input set, rewrite the stored path as relative to the archive root. Requires:

- A normalization step during file walk that maps absolute paths → archive entry names.
- Either a marker in the entry (private `fLTP` value in range 64–255, or an ancillary chunk) or a convention tying relative paths to archive-internal semantics and absolute paths to external semantics.
- No change needed at EXTRACT time: the extract branch added in Task 4.1 already handles both absolute and relative stored paths.

This is explicitly deferred. When the base junction support merges, open a separate plan using the same spec shape.

**Expose reparse-point types publicly for downstream consumers.** If a third-party crate wants to reuse `ReparsePoint` / `read_reparse_point` from the CLI, the items could be promoted into `pna` behind a new feature flag (e.g. `pna` with `windows-fs`). For now they stay CLI-private because `pna` must not carry a platform-specific build dependency.

---

## Self-Review Checklist

**Spec coverage:**
- [x] Junction detection on Windows (Task 1.4, Task 3.1)
- [x] Recording as HardLink + fLTP=Directory (Task 3.1)
- [x] Absolute target encoding (`\??\` stripped, Task 1.1)
- [x] Extract on Windows → junction (Task 4.1, 4.3)
- [x] Extract on non-Windows → symlink fallback (Task 4.1, 4.2)
- [x] Extract accepts both absolute and relative stored targets (Task 4.1)
- [x] `--allow-unsafe-links` gating (Task 4.1, 4.2)
- [x] Unknown reparse tag → debug log (Task 3.1 Step 5, Task 5.1)
- [x] Regression-free for existing hardlink/symlink (Task 3.1 Step 9, Task 4.1 Step 5, Task 5.2)
- [x] Docs update (Task 6.1)
- [x] `PathnameEditor::edit_junction` with shared helper (Task 2.1)
- [x] No `pna` / `libpna` modifications (by construction)

**Placeholder scan:** no `TBD` / `TODO` / "similar to task N". Code blocks show full implementations.

**Type consistency:**
- `ReparsePoint` variants are identical in Phase 1 tests and Phase 4 tests.
- `StoreAs::Junction(PathBuf)` matches in create (Task 3.1) and any exhaustive matches (Task 3.1 Step 8, Task 5.2).
- `LinkTargetType::Directory` is the fLTP value throughout.
- `detect_junction` returns `io::Result<Option<PathBuf>>` consistently.
- `PathnameEditor::edit_junction` returns `EntryReference`, identical to `edit_symlink`.
- `create_junction_or_fallback` takes `(&Path, &str)`; callers pass `transformed.as_str()`.

**Feedback memory check:**
- `feedback_plan_for_plans` ✓ (this plan was itself adversarially grilled before writing).
- `feedback_no_impl_without_plan` ✓ (plan produced before any code).
- `feedback_test_matrix_first` ✓ (matrix at top of file, 18 cases).
- `feedback_stop_at_push` ✓ (Task 6.2 Step 6 stops at push).
- `feedback_check_ci_after_push` ✓ (Task 6.2 Step 6).
- `feedback_subagent_opus_only` ✓ (subagents for implementation/review dispatched with `model: "opus"`).
- `feedback_safe_bulk_edit` ✓ (no `replace_all` in the plan; all edits are targeted).
- `feedback_confirm_irreversible` ✓ (no destructive operations beyond the final `git push`).
- `feedback_verify_claims` ✓ (`preserve_root` API and `windows` crate features were verified via source / `cargo check` before committing this plan).
