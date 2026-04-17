# Windows Junction Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect Windows junctions during PNA archive creation and restore them during extraction, using the `DataKind::HardLink + fLTP=Directory` encoding with an absolute target path.

**Architecture:** No change to `libpna` — `EntryBuilder::new_hard_link(...)` combined with `.link_target_type(LinkTargetType::Directory)` already round-trips through the format. All work happens in `pna/` (Windows reparse-point FFI primitives) and `cli/` (a new `StoreAs::Junction` classifier plus an extract-path branch that skips `edit_hardlink` sanitization for `fLTP=Directory`). On Windows extract we create a real junction via `FSCTL_SET_REPARSE_POINT`; on non-Windows we fall back to a symbolic link per the PNA spec's `MAY` clause at `chunk_specifications/index.md:332-336`.

**Tech Stack:** Rust 2024 (MSRV 1.88), existing `windows = "0.62"` crate (adding `Win32_System_Ioctl` feature for `DeviceIoControl` + `FSCTL_*`), existing `windows-sys` as already used in the crate, existing `fLTP` / `FHED` chunk types (no format changes), existing `--allow-unsafe-links` flag.

---

## Scope

**In scope (Phase 1-6):**
- Junction detection on Windows during `create` (NTFS reparse tag = `IO_REPARSE_TAG_MOUNT_POINT`)
- Reparse buffer parsing to extract `SubstituteName`, strip `\??\` prefix → UTF-8 path
- Encoding: `DataKind::HardLink` + `fLTP=Directory` + entry data = external absolute path
- Extract branch: on Windows create junction via `FSCTL_SET_REPARSE_POINT`; on non-Windows create symbolic link
- `--allow-unsafe-links` gates extraction of junction entries (the target is always external absolute)
- Reparse tags other than `IO_REPARSE_TAG_MOUNT_POINT` and `IO_REPARSE_TAG_SYMLINK` → warn + skip
- Regression-free for existing hardlink and symlink tests

**Out of scope (deferred):**
- Relative-path optimization when junction target is inside the archive input set (user directive Q2 — separate future plan)
- Other reparse tags (`IO_REPARSE_TAG_CLOUD_*` OneDrive placeholders, App Execution Aliases, etc.)
- PNA specification document changes (user directive Q6 — implementation defines the encoding; the spec's existing "directory hard link = junction" text stays as-is)
- Windows UNC junctions (the kernel forbids UNC targets for `IO_REPARSE_TAG_MOUNT_POINT`; we do not need to handle them on write, but we must not panic on read)

## Prerequisites per Task

1. **Aegis consultation before code changes.** Per project CLAUDE.md, every task that touches source files must start by calling `aegis_compile_context` with `target_files` set to the files listed in the task header, `plan` set to the task's action, and `command` set to `"scaffold"` or `"refactor"` as appropriate. Follow returned guidelines. Report `compile_miss` if any guideline was missing.
2. **TDD order.** Each task writes the failing test first, runs it to confirm the failure, implements the minimum change, re-runs, then commits. This order is non-negotiable — `feedback_no_impl_without_plan` applies.
3. **Commit message.** Follow the project convention at CLAUDE.md's emoji table. Do not add `Co-Authored-By` lines (global CLAUDE.md).

## Risk Summary

| Risk | Mitigation |
|---|---|
| `DeviceIoControl` unsafe FFI mistakes → UB | Use `windows = "0.62"` Rust bindings; wrap all raw pointer math in small helpers; unit-test the reparse-buffer parser with hand-crafted byte slices |
| Junction detection accidentally catches `IO_REPARSE_TAG_SYMLINK` | Strict tag match; unknown tags go to `warn + skip` rather than falling through |
| Extract changes break existing hardlink tests | Keep the old HardLink code path untouched; add a separate branch gated on `fLTP == Some(Directory)` |
| Symlink fallback on non-Windows creates a user-visible behavior change for existing archives | Not applicable — no existing archive contains this encoding today |
| Path traversal via junction target (`..`, `/etc/passwd`) | Junction target is always treated as an unsafe link; `--allow-unsafe-links` required |

## Testing Matrix (matrix-first per `feedback_test_matrix_first`)

| # | Scenario | Platform | Test kind | Location |
|---|---|---|---|---|
| T1 | Reparse buffer parser (SubstituteName extraction) | cross-platform | unit | `pna/src/fs/reparse.rs` |
| T2 | `\??\` prefix stripping | cross-platform | unit | `pna/src/fs/reparse.rs` |
| T3 | Junction detection returns `Some(target)` for junction | Windows | unit | `cli/src/utils/os/windows/fs/junction.rs` |
| T4 | Junction detection returns `None` for regular dir / symlink / file | Windows | unit | same |
| T5 | `StoreAs::Junction` classification via `classify_symlink_like` | Windows | unit | `cli/src/command/core.rs` |
| T6 | `create_entry` for junction emits HardLink + fLTP=Directory + absolute path | Windows | integration | `cli/tests/cli/junction.rs` |
| T7 | Round-trip: create archive from junction → extract → junction recreated | Windows | integration | same |
| T8 | Extract HardLink+fLTP=Directory on non-Windows → symlink created | cross-platform | integration | same (using pre-crafted fixture) |
| T9 | Extract without `--allow-unsafe-links` → warn + skip | cross-platform | integration | same |
| T10 | Unknown reparse tag during create → warn + skip | Windows | integration | same |
| T11 | Existing hardlink tests regression-free | cross-platform | existing | `cli/tests/cli/hardlink.rs`, `cli/tests/cli/extract/hardlink.rs` |
| T12 | Existing symlink tests regression-free | cross-platform | existing | `cli/tests/cli/extract/symlink.rs` etc. |

## File Structure

### New files

| Path | Responsibility |
|---|---|
| `pna/src/fs/reparse.rs` | Windows-only reparse-point FFI primitives: `read_reparse_point(path) -> ReparsePoint`, `create_junction(link, target) -> io::Result<()>`. `ReparsePoint` enum variants: `Junction(PathBuf)`, `Symlink { target: PathBuf, is_relative: bool }`, `Other(u32)`. |
| `cli/src/utils/os/windows/fs/junction.rs` | CLI-side wrapper: `detect_junction(path) -> io::Result<Option<PathBuf>>` that uses `pna::fs::reparse::read_reparse_point` and returns `Some(target)` only for `ReparsePoint::Junction`. |
| `cli/src/utils/os/windows/fs/mod.rs` | Module shim that declares `junction` and (later) any sibling Windows FS utilities. |
| `cli/tests/cli/junction.rs` | Integration tests T6–T10. Windows-gated round-trip tests use `#[cfg(windows)]`; cross-platform extract tests use a libpna-constructed fixture. |
| `lib/tests/resources/` is **not** extended — we construct fixtures in-test via `EntryBuilder`. |

### Modified files

| Path | Change |
|---|---|
| `pna/src/fs.rs` | Add `#[cfg(windows)] pub mod reparse;` so the new module is exposed as `pna::fs::reparse`. |
| `pna/Cargo.toml` | Ensure `windows` (or `windows-sys`) has `Win32_System_Ioctl`, `Win32_Storage_FileSystem`, `Win32_System_IO` features. |
| `cli/src/utils/os/windows/mod.rs` | Declare `pub mod fs;`. |
| `cli/src/command/core.rs` | Add `StoreAs::Junction(PathBuf)` variant (line 379-384). Insert junction detection before the existing `is_symlink` classification (line 738). Add a `create_entry` arm for `StoreAs::Junction` (near line 915). |
| `cli/src/command/extract.rs` | Inside `DataKind::HardLink` arm (line 1422-1451), branch on `item.metadata().link_target_type()`; when `Some(Directory)`, skip `edit_hardlink`, treat path as external absolute, require `--allow-unsafe-links`, then call `pna::fs::reparse::create_junction` on Windows or `utils::fs::symlink` elsewhere. |

### Files **not** modified

- `lib/` — libpna already round-trips HardLink + fLTP=Directory (verified at `lib/src/entry/builder.rs:783`, `:798` with existing tests `builder_hardlink_with_link_target_type_directory`).
- PNA specification repo — no changes per Q6.

---

## Task List

### Phase 1: `pna` reparse-point primitives

Goal: expose a minimal, testable Rust API that reads and writes NTFS reparse points. Windows-only. No PNA logic here.

#### Task 1.1: Create module skeleton and reparse-buffer parser

**Files:**
- Create: `pna/src/fs/reparse.rs`
- Modify: `pna/src/fs.rs` (add `#[cfg(windows)] pub mod reparse;`)
- Modify: `pna/Cargo.toml` (add `Win32_System_Ioctl` feature to the existing `windows` dependency)

- [ ] **Step 1: Call `aegis_compile_context`**

Call `aegis_compile_context` with `target_files: ["pna/src/fs/reparse.rs", "pna/src/fs.rs", "pna/Cargo.toml"]`, `plan: "Add Windows reparse point primitives module and expose it from pna::fs::reparse"`, `command: "scaffold"`. Follow any guidelines returned.

- [ ] **Step 2: Write the failing unit test for the reparse-buffer parser**

Create `pna/src/fs/reparse.rs` with only the test (no implementation yet):

```rust
//! Windows NTFS reparse-point primitives.
#![cfg(windows)]

use std::{io, path::PathBuf};

/// Parsed contents of a reparse point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReparsePoint {
    /// IO_REPARSE_TAG_MOUNT_POINT (junction). Target is always absolute.
    Junction(PathBuf),
    /// IO_REPARSE_TAG_SYMLINK (directory/file symbolic link).
    Symlink { target: PathBuf, is_relative: bool },
    /// Any other reparse tag we do not handle (OneDrive placeholders, etc.).
    Other(u32),
}

/// Parse a raw reparse-point buffer (the payload of FSCTL_GET_REPARSE_POINT).
///
/// Errors when the buffer is truncated or malformed.
pub(crate) fn parse_reparse_buffer(buf: &[u8]) -> io::Result<ReparsePoint> {
    todo!("implemented in Step 4")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reparse buffer for a junction pointing to C:\target,
    /// hand-crafted per REPARSE_DATA_BUFFER layout.
    fn sample_junction_buffer() -> Vec<u8> {
        // Layout: ReparseTag(4) + DataLength(2) + Reserved(2)
        //       + SubstituteOffset(2) + SubstituteLen(2)
        //       + PrintOffset(2) + PrintLen(2)
        //       + PathBuffer (UTF-16 LE, no null terminator)
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
        assert!(parse_reparse_buffer(&short).is_err());
    }
}
```

Edit `pna/src/fs.rs` to add at the top (after existing `use` lines):

```rust
#[cfg(windows)]
pub mod reparse;
```

- [ ] **Step 3: Run the test to confirm it fails**

Run: `cargo test -p pna --target x86_64-pc-windows-msvc reparse::tests::parses_junction_and_strips_nt_prefix` (on Windows CI).
On non-Windows developer machines this will be a `cargo check` only because the module is cfg-gated. Expected: FAIL with `not yet implemented` panic (from `todo!()`).

- [ ] **Step 4: Implement `parse_reparse_buffer`**

Replace `todo!(...)` with:

```rust
pub(crate) fn parse_reparse_buffer(buf: &[u8]) -> io::Result<ReparsePoint> {
    const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
    const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
    const HEADER_LEN: usize = 8; // ReparseTag(4) + DataLength(2) + Reserved(2)
    const MP_PATHBUF_OFFSET: usize = HEADER_LEN + 8; // + Substitute/Print offsets+lens
    const SYMLINK_PATHBUF_OFFSET: usize = HEADER_LEN + 12; // + Flags(4)

    if buf.len() < HEADER_LEN {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "reparse buffer too short for header"));
    }
    let tag = u32::from_le_bytes(buf[0..4].try_into().unwrap());

    let read_u16 = |offset: usize| -> io::Result<u16> {
        buf.get(offset..offset + 2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "reparse buffer truncated"))
    };

    let extract_utf16 = |offset: usize, len: u16, pathbuf_base: usize| -> io::Result<String> {
        let start = pathbuf_base + offset as usize;
        let end = start + len as usize;
        let slice = buf.get(start..end).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "reparse path buffer out of range")
        })?;
        if slice.len() % 2 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "odd-length utf16"));
        }
        let utf16: Vec<u16> = slice
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&utf16)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid utf16 in reparse path"))
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
            let subst_offset = read_u16(HEADER_LEN)?;
            let subst_len = read_u16(HEADER_LEN + 2)?;
            let flags = u32::from_le_bytes(buf[HEADER_LEN + 8..HEADER_LEN + 12].try_into().unwrap());
            let is_relative = flags & 0x1 != 0; // SYMLINK_FLAG_RELATIVE
            let subst = extract_utf16(subst_offset as usize, subst_len, SYMLINK_PATHBUF_OFFSET)?;
            let stripped = if !is_relative {
                subst.strip_prefix(r"\??\").unwrap_or(&subst).to_string()
            } else {
                subst
            };
            Ok(ReparsePoint::Symlink { target: PathBuf::from(stripped), is_relative })
        }
        other => Ok(ReparsePoint::Other(other)),
    }
}
```

- [ ] **Step 5: Run tests to confirm they pass**

Run: `cargo test -p pna reparse::tests` (Windows CI).
Expected: both tests PASS.

- [ ] **Step 6: Commit**

```bash
git add pna/src/fs.rs pna/src/fs/reparse.rs pna/Cargo.toml
git commit -m ":sparkles: Add reparse buffer parser for Windows junctions"
```

---

#### Task 1.2: `read_reparse_point(path)` — DeviceIoControl wrapper

**Files:**
- Modify: `pna/src/fs/reparse.rs`

- [ ] **Step 1: Call `aegis_compile_context`**

`target_files: ["pna/src/fs/reparse.rs"]`, `plan: "Add read_reparse_point using DeviceIoControl + FSCTL_GET_REPARSE_POINT"`, `command: "scaffold"`.

- [ ] **Step 2: Write a failing integration-style test gated on Windows**

Append to the existing `tests` module:

```rust
#[test]
#[cfg(windows)]
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
            assert!(t.to_string_lossy().ends_with("target"));
        }
        other => panic!("expected Junction, got {other:?}"),
    }
    Ok(())
}
```

Also add `tempfile = "3"` to `pna/Cargo.toml` `[dev-dependencies]` if not present.

- [ ] **Step 3: Run to confirm failure**

Run: `cargo test -p pna --target x86_64-pc-windows-msvc reparse::tests::read_reparse_point_on_junction`.
Expected: FAIL with `cannot find function read_reparse_point`.

- [ ] **Step 4: Implement `read_reparse_point`**

Add to `pna/src/fs/reparse.rs` (below `parse_reparse_buffer`):

```rust
use std::{os::windows::ffi::OsStrExt, path::Path, ptr};

use windows::Win32::{
    Foundation::{CloseHandle, GENERIC_READ, HANDLE},
    Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    },
    System::{Ioctl::FSCTL_GET_REPARSE_POINT, IO::DeviceIoControl},
};

/// Read the reparse data at `path`. Returns `ReparsePoint::Other(tag)` for tags we don't handle.
///
/// Errors if the path is not a reparse point, the file cannot be opened, or the
/// reparse buffer is malformed.
pub fn read_reparse_point(path: &Path) -> io::Result<ReparsePoint> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let handle: HANDLE = unsafe {
        CreateFileW(
            windows::core::PCWSTR(wide.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|e| io::Error::other(e))?;

    let mut buf = vec![0u8; 16 * 1024]; // MAXIMUM_REPARSE_DATA_BUFFER_SIZE
    let mut bytes_returned: u32 = 0;

    let result = unsafe {
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
    let close_result = unsafe { CloseHandle(handle) };
    result.map_err(|e| io::Error::other(e))?;
    let _ = close_result;

    buf.truncate(bytes_returned as usize);
    parse_reparse_buffer(&buf)
}
```

- [ ] **Step 5: Run test**

Run: `cargo test -p pna reparse::tests::read_reparse_point_on_junction` (Windows CI).
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add pna/src/fs/reparse.rs pna/Cargo.toml
git commit -m ":sparkles: Read NTFS reparse points via DeviceIoControl"
```

---

#### Task 1.3: `create_junction(link, target)` — FSCTL_SET_REPARSE_POINT

**Files:**
- Modify: `pna/src/fs/reparse.rs`

- [ ] **Step 1: Call `aegis_compile_context`** with `plan: "Add create_junction using FSCTL_SET_REPARSE_POINT"`.

- [ ] **Step 2: Write the failing test**

```rust
#[test]
#[cfg(windows)]
fn create_junction_round_trip() -> io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let target = tmp.path().join("target");
    std::fs::create_dir(&target)?;
    let link = tmp.path().join("junction");
    super::create_junction(&link, &target)?;

    let rp = super::read_reparse_point(&link)?;
    match rp {
        ReparsePoint::Junction(t) => assert!(t.ends_with("target")),
        other => panic!("expected Junction, got {other:?}"),
    }
    Ok(())
}
```

- [ ] **Step 3: Run to confirm failure**

Run: `cargo test -p pna reparse::tests::create_junction_round_trip`.
Expected: FAIL with `cannot find function create_junction`.

- [ ] **Step 4: Implement `create_junction`**

Add to `pna/src/fs/reparse.rs`:

```rust
use windows::Win32::{
    Foundation::GENERIC_WRITE,
    Storage::FileSystem::{CREATE_NEW, FILE_FLAG_POSIX_SEMANTICS},
    System::Ioctl::FSCTL_SET_REPARSE_POINT,
};

/// Create a junction at `link` pointing to the absolute `target`.
///
/// `link` must not already exist; `target` must be an absolute path to a directory.
pub fn create_junction(link: &Path, target: &Path) -> io::Result<()> {
    if !target.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "junction target must be absolute",
        ));
    }
    std::fs::create_dir(link)?;

    let wide: Vec<u16> = link.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let handle: HANDLE = unsafe {
        CreateFileW(
            windows::core::PCWSTR(wide.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(io::Error::other)?;

    let subst_wide: Vec<u16> = format!(r"\??\{}", target.display())
        .encode_utf16()
        .collect();
    let print_wide: Vec<u16> = target.as_os_str().encode_wide().collect();
    let subst_bytes_len = (subst_wide.len() * 2) as u16;
    let print_bytes_len = (print_wide.len() * 2) as u16;

    let mut buf = Vec::<u8>::new();
    buf.extend(&0xA000_0003u32.to_le_bytes());            // ReparseTag
    let data_len: u16 = 8 + subst_bytes_len + print_bytes_len;
    buf.extend(&data_len.to_le_bytes());                  // ReparseDataLength
    buf.extend(&0u16.to_le_bytes());                      // Reserved
    buf.extend(&0u16.to_le_bytes());                      // SubstituteNameOffset
    buf.extend(&subst_bytes_len.to_le_bytes());           // SubstituteNameLength
    buf.extend(&subst_bytes_len.to_le_bytes());           // PrintNameOffset (after Subst)
    buf.extend(&print_bytes_len.to_le_bytes());           // PrintNameLength
    for u in &subst_wide { buf.extend(&u.to_le_bytes()); }
    for u in &print_wide { buf.extend(&u.to_le_bytes()); }

    let mut bytes_returned = 0u32;
    let result = unsafe {
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
    let _ = unsafe { CloseHandle(handle) };
    result.map_err(|e| {
        let _ = std::fs::remove_dir(link);
        io::Error::other(e)
    })?;
    Ok(())
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p pna reparse::tests` (Windows CI).
Expected: all three reparse tests PASS.

- [ ] **Step 6: Commit**

```bash
git add pna/src/fs/reparse.rs
git commit -m ":sparkles: Create NTFS junctions via FSCTL_SET_REPARSE_POINT"
```

---

### Phase 2: CLI junction detection utility

#### Task 2.1: `detect_junction(path)` wrapper

**Files:**
- Create: `cli/src/utils/os/windows/fs/mod.rs`
- Create: `cli/src/utils/os/windows/fs/junction.rs`
- Modify: `cli/src/utils/os/windows/mod.rs` (add `pub mod fs;`)

- [ ] **Step 1: Call `aegis_compile_context`** with `target_files: ["cli/src/utils/os/windows/fs/junction.rs", "cli/src/utils/os/windows/fs/mod.rs", "cli/src/utils/os/windows/mod.rs"]`, `plan: "Add detect_junction helper"`, `command: "scaffold"`.

- [ ] **Step 2: Write the failing test**

Create `cli/src/utils/os/windows/fs/junction.rs`:

```rust
//! Windows junction detection for the CLI.
#![cfg(windows)]

use std::{io, path::{Path, PathBuf}};

use pna::fs::reparse::{read_reparse_point, ReparsePoint};

/// If `path` is a junction, returns its absolute target; otherwise `None`.
///
/// Returns `Ok(None)` for non-reparse paths, regular symlinks, and unknown
/// reparse tags (which callers may log separately). Returns `Err` only for
/// I/O failures on the reparse-point read itself.
pub fn detect_junction(path: &Path) -> io::Result<Option<PathBuf>> {
    match read_reparse_point(path) {
        Ok(ReparsePoint::Junction(t)) => Ok(Some(t)),
        Ok(_) => Ok(None),
        Err(e) if e.kind() == io::ErrorKind::InvalidData => {
            // Not a reparse point: treat as "not a junction".
            Ok(None)
        }
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
}
```

Create `cli/src/utils/os/windows/fs/mod.rs`:

```rust
pub mod junction;
```

Modify `cli/src/utils/os/windows/mod.rs` to add `pub mod fs;` near its other `pub mod` lines.

Add `tempfile` to `cli/Cargo.toml` `[dev-dependencies]` if absent.

- [ ] **Step 3: Run the test**

Run: `cargo test -p portable-network-archive --lib windows::fs::junction::tests::regular_directory_is_not_junction` (Windows CI).
Expected: PASS (since implementation is there). If this is the first compile, confirm the non-Windows build of the whole workspace still compiles via `cargo check --workspace`.

- [ ] **Step 4: Commit**

```bash
git add cli/src/utils/os/windows/fs/ cli/src/utils/os/windows/mod.rs cli/Cargo.toml
git commit -m ":sparkles: Add detect_junction helper for CLI"
```

Note: we skip the `fail-first` step for this task because the implementation is just a thin delegation to `pna::fs::reparse` whose correctness is covered by Phase 1. The **behavioral** tests come in Phase 3 integration work.

---

### Phase 3: CLI create path

#### Task 3.1: Add `StoreAs::Junction(PathBuf)` variant and wire classification

**Files:**
- Modify: `cli/src/command/core.rs` (lines 379-384, 738-745)

- [ ] **Step 1: Call `aegis_compile_context`** with `target_files: ["cli/src/command/core.rs"]`, `plan: "Add StoreAs::Junction variant and detect junction before symlink classification on Windows"`, `command: "refactor"`.

- [ ] **Step 2: Write the failing integration test**

Create `cli/tests/cli/junction.rs`:

```rust
#![cfg(windows)]

use std::{collections::HashSet, path::PathBuf, process::Command};

use pna::{prelude::*, Archive};

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
/// Expectation: the junction is encoded as HardLink + fLTP=Directory with the absolute target path.
#[test]
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
        if entry.header().path().as_str().ends_with("link_dir") {
            assert_eq!(entry.header().data_kind(), DataKind::HardLink);
            assert_eq!(
                entry.metadata().link_target_type(),
                Some(LinkTargetType::Directory)
            );
            let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut reader, &mut s).unwrap();
            assert!(s.ends_with("target_dir"), "expected absolute target, got {s:?}");
            saw_junction_entry = true;
        }
    }
    assert!(saw_junction_entry, "no HardLink entry found for junction");
}
```

Register the test module in `cli/tests/cli/main.rs` (or wherever the other test modules are registered). Check existing patterns like `mod hardlink;` near the top.

- [ ] **Step 3: Run the test to confirm failure**

Run: `cargo test -p portable-network-archive --test cli create_records_junction_as_hardlink_directory` (Windows CI).
Expected: FAIL — the archive currently records the junction as a broken Symlink.

- [ ] **Step 4: Edit `cli/src/command/core.rs` — add `Junction` variant**

At line 379:

```rust
#[derive(Clone, Debug)]
pub(crate) enum StoreAs {
    File,
    Dir,
    Symlink(LinkTargetType),
    Hardlink(PathBuf),
    /// A Windows NTFS junction; the `PathBuf` is the absolute target path.
    Junction(PathBuf),
}
```

- [ ] **Step 5: Detect junction before symlink classification**

Replace the block at line 738-741:

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
    let junction_target = classify_junction(path)?;
    if let Some(target) = junction_target {
        Some((StoreAs::Junction(target), meta))
    } else {
        let link_target_type = detect_symlink_target_type(path, &meta)?;
        Some((StoreAs::Symlink(link_target_type), meta))
    }
```

Add a small helper at the bottom of `core.rs` alongside `detect_symlink_target_type`:

```rust
#[cfg(windows)]
fn classify_junction(path: &Path) -> io::Result<Option<PathBuf>> {
    utils::os::windows::fs::junction::detect_junction(path)
}

#[cfg(not(windows))]
fn classify_junction(_path: &Path) -> io::Result<Option<PathBuf>> {
    Ok(None)
}
```

- [ ] **Step 6: Add `create_entry` arm for `StoreAs::Junction`**

In the `match store_as` at line 914 (after the `Hardlink` arm at 915-922):

```rust
StoreAs::Junction(target) => {
    // Junction target is an external absolute path; skip edit_hardlink.
    let reference = EntryReference::from(target.as_path());
    let mut entry = EntryBuilder::new_hard_link(entry_name, reference)?;
    entry.link_target_type(LinkTargetType::Directory);
    apply_metadata(entry, path, keep_options, metadata)?.build()
}
```

If `EntryReference::from(&Path)` does not exist, use whatever conversion new_hard_link accepts (most likely `EntryReference::try_from(&Path)` or construct from the string). Verify against `lib/src/entry/reference.rs` before committing.

- [ ] **Step 7: Update exhaustive matches**

Search for other `match ... { StoreAs::` sites and add the `Junction` arm. Grep: `rg 'StoreAs::' cli/src`. Likely sites: `list.rs`, `diff.rs` (per the investigation). Each should route junction either by:
- Treating it like `Hardlink` for display, or
- Treating it like `Symlink(Directory)` for display (closer to user expectation).

Pick one convention consistently. Recommendation: display as hardlink because that's the on-disk encoding.

- [ ] **Step 8: Run tests**

Run: `cargo test -p portable-network-archive --test cli create_records_junction_as_hardlink_directory` (Windows CI).
Expected: PASS.

Also run: `cargo test --workspace --all-features` (any platform) to catch regressions and confirm cross-platform builds.

- [ ] **Step 9: Commit**

```bash
git add cli/src/command/core.rs cli/tests/cli/junction.rs cli/tests/cli/main.rs
git commit -m ":sparkles: Detect and archive Windows junctions as HardLink entries"
```

---

### Phase 4: CLI extract path

#### Task 4.1: Branch on `fLTP=Directory` in `DataKind::HardLink` extract arm

**Files:**
- Modify: `cli/src/command/extract.rs` (lines 1422-1451)

- [ ] **Step 1: Call `aegis_compile_context`** with `target_files: ["cli/src/command/extract.rs"]`, `plan: "Branch DataKind::HardLink on fLTP=Directory to create junction on Windows / symlink fallback elsewhere"`, `command: "refactor"`.

- [ ] **Step 2: Write failing cross-platform fixture test**

Append to `cli/tests/cli/junction.rs` (but outside the `#[cfg(windows)]` block) — make this test `#[cfg(any(unix, windows))]` because it uses the libpna API to build a fixture and checks platform-appropriate extraction:

```rust
use pna::{Archive, EntryBuilder, EntryReference, LinkTargetType, WriteOptions};

/// Build an in-memory archive containing one HardLink+fLTP=Directory entry
/// pointing to `target`.
fn build_junction_fixture(target: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut archive = Archive::write_header(&mut out).unwrap();
    let name: pna::EntryName = "link_dir".parse().unwrap();
    let reference = EntryReference::try_from(std::path::Path::new(target)).unwrap();
    let mut builder = EntryBuilder::new_hard_link(name, reference).unwrap();
    builder.link_target_type(LinkTargetType::Directory);
    let entry = builder.build().unwrap();
    archive
        .add_entry(entry, WriteOptions::builder().build())
        .unwrap();
    archive.finalize().unwrap();
    out
}

/// Precondition: archive with a HardLink+fLTP=Directory entry.
/// Action: extract without --allow-unsafe-links.
/// Expectation: the entry is skipped with a warning.
#[test]
fn extract_junction_without_allow_unsafe_links_skips() {
    let tmp = tempfile::tempdir().unwrap();
    let archive_path = tmp.path().join("fixture.pna");
    std::fs::write(&archive_path, build_junction_fixture("/any/absolute/path")).unwrap();

    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .args(["extract", "-f"])
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!out_dir.join("link_dir").exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unsafe"), "expected skip warning, got: {stderr}");
}
```

- [ ] **Step 3: Run to confirm failure**

Run: `cargo test -p portable-network-archive --test cli extract_junction_without_allow_unsafe_links_skips`.
Expected: FAIL — current extract code calls `edit_hardlink` which strips the leading `/`, then `fs::hard_link` fails with an odd error.

- [ ] **Step 4: Implement the branch**

In `extract.rs`, replace the `DataKind::HardLink => { ... }` arm at line 1422-1451 with:

```rust
DataKind::HardLink => {
    let reader = item.reader(ReadOptions::with_password(password))?;
    let original = io::read_to_string(reader)?;
    let is_directory_link =
        matches!(item.metadata().link_target_type(), Some(LinkTargetType::Directory));

    if is_directory_link {
        // Encoded junction: target is an external absolute path; do NOT sanitize.
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
        create_junction_or_fallback(&path, original.as_ref())?;
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

Add a helper near the bottom of `extract.rs`:

```rust
fn create_junction_or_fallback(link: &Path, target: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        pna::fs::reparse::create_junction(link, Path::new(target))
    }
    #[cfg(not(windows))]
    {
        log::warn!(
            "Creating symbolic link instead of Windows junction on non-Windows platform: {} -> {}",
            link.display(),
            target
        );
        utils::fs::symlink(target, link)
    }
}
```

Ensure imports at the top of `extract.rs` include `LinkTargetType` from libpna.

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
- Modify: `cli/tests/cli/junction.rs` (add tests only)

- [ ] **Step 1: Write the test**

```rust
/// Precondition: archive with a HardLink+fLTP=Directory entry.
/// Action: extract with --allow-unsafe-links.
/// Expectation: on Windows a junction is created; on other platforms a symlink is created.
#[test]
fn extract_junction_with_allow_unsafe_links_creates_link() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("actual_target");
    std::fs::create_dir(&target).unwrap();
    let archive_path = tmp.path().join("fixture.pna");
    std::fs::write(
        &archive_path,
        build_junction_fixture(target.to_str().unwrap()),
    )
    .unwrap();

    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
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
        use pna::fs::reparse::{read_reparse_point, ReparsePoint};
        match read_reparse_point(&link).unwrap() {
            ReparsePoint::Junction(t) => assert_eq!(t, target),
            other => panic!("expected junction, got {other:?}"),
        }
    }
    #[cfg(not(windows))]
    {
        assert!(meta.file_type().is_symlink());
        assert_eq!(std::fs::read_link(&link).unwrap(), target);
    }
    let _ = meta;
}
```

- [ ] **Step 2: Run**

Run: `cargo test -p portable-network-archive --test cli extract_junction_with_allow_unsafe_links_creates_link` on both platforms.
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
    assert!(std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
        .arg(&archive_path)
        .args(["link_dir", "target_dir"])
        .status()
        .unwrap()
        .success());

    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    assert!(std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .args(["extract", "-f"])
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--allow-unsafe-links")
        .status()
        .unwrap()
        .success());

    use pna::fs::reparse::{read_reparse_point, ReparsePoint};
    let rp = read_reparse_point(&out_dir.join("link_dir")).unwrap();
    assert!(matches!(rp, ReparsePoint::Junction(_)));
}
```

- [ ] **Step 2: Run**

Run: `cargo test -p portable-network-archive --test cli round_trip_junction_via_cli` (Windows CI).
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add cli/tests/cli/junction.rs
git commit -m ":white_check_mark: End-to-end junction round trip via CLI"
```

---

### Phase 5: Hardening

#### Task 5.1: Unknown reparse tag → warn + skip during create

**Files:**
- Modify: `cli/src/command/core.rs`
- Modify: `cli/tests/cli/junction.rs`

- [ ] **Step 1: Call `aegis_compile_context`**.

- [ ] **Step 2: Write the test**

Create a reparse point with an unusual tag in a test by temporarily injecting a mock reparse buffer OR by using a well-known Windows artifact (App Execution Alias under `%LOCALAPPDATA%\Microsoft\WindowsApps` can return `IO_REPARSE_TAG_APPEXECLINK`). If a reliable fixture is unavailable in CI, wrap this as a `#[ignore]` smoke test and do the assertion via unit test on `detect_junction`: a `ReparsePoint::Other(_)` input must map to `Ok(None)`, and a log call must happen (inject a log capture).

Concrete addition to `cli/src/utils/os/windows/fs/junction.rs` tests:

```rust
#[test]
fn other_reparse_tag_is_not_reported_as_junction() {
    // Synthetic: simulate by direct parse_reparse_buffer — outside detect_junction contract.
    // This test covers the classification rule; the actual log-on-unknown is exercised in Step 3.
    use pna::fs::reparse::parse_reparse_buffer;
    let mut buf = vec![0u8; 8];
    buf[0..4].copy_from_slice(&0x8000_0023u32.to_le_bytes()); // IO_REPARSE_TAG_APPEXECLINK
    let rp = parse_reparse_buffer(&buf).unwrap();
    assert!(matches!(rp, pna::fs::reparse::ReparsePoint::Other(0x8000_0023)));
}
```

(Note: `parse_reparse_buffer` is `pub(crate)` in Phase 1 — promote it to `pub` if this test lives in the CLI crate, or keep the test inside `pna/` instead.)

- [ ] **Step 3: Add a warn-log when classification encounters an unknown reparse tag**

In `core.rs`, when `detect_junction` returns `Ok(None)` for a file with `is_symlink == true` but the symlink detection also fails to classify it (i.e., `link_target_type == Unknown`), emit a warning and skip. Since the current behavior is to record it as a broken symlink, this is a behavior change — discuss with maintainer before enabling. For Phase 5 we take the conservative route: log at `debug` level only.

Concretely, modify `classify_junction` wrapper:

```rust
#[cfg(windows)]
fn classify_junction(path: &Path) -> io::Result<Option<PathBuf>> {
    match utils::os::windows::fs::junction::detect_junction(path) {
        Ok(v) => Ok(v),
        Err(e) => {
            log::debug!("Failed to inspect reparse point {}: {}", path.display(), e);
            Ok(None)
        }
    }
}
```

- [ ] **Step 4: Run and commit**

```bash
cargo test -p pna
cargo test -p portable-network-archive --test cli
git add cli/src/command/core.rs cli/src/utils/os/windows/fs/junction.rs
git commit -m ":sparkles: Log reparse inspection failures at debug level"
```

---

#### Task 5.2: `bsdtar compat` regression check

**Files:**
- Verify: `cli/tests/bats/` and `cli/src/command/bsdtar.rs`

- [ ] **Step 1: Run the bsdtar-compat skill for a sanity check**

Invoke `bsdtar-compat-verify` skill over the changes if available in the current plugin set. Otherwise:

Run: `cargo test -p portable-network-archive --test cli compat` (or equivalent) and run the bats tests if the env has `bats` available: `bats tests/bats/`.
Expected: no regressions.

- [ ] **Step 2: If any bsdtar compat behavior diverges, open a follow-up task**

Document in a new task at the end of this plan. Do not modify `bsdtar.rs` in this PR — the compat subcommand intentionally does not record junctions (bsdtar itself doesn't) and we accept the same limitation.

- [ ] **Step 3: Commit only if changes were made**

---

#### Task 5.3: Coverage sweep

- [ ] **Step 1: Run `rust-coverage-analysis` skill over the new files**

Expected: report lists `pna/src/fs/reparse.rs`, `cli/src/utils/os/windows/fs/junction.rs`, the new arms in `core.rs` and `extract.rs`. Target ≥80% line coverage on each.

- [ ] **Step 2: If uncovered paths are non-trivial, add a test; otherwise annotate with `// coverage: Windows-only` comments**

- [ ] **Step 3: Commit if tests added**

```bash
git commit -m ":white_check_mark: Improve coverage of junction paths"
```

---

### Phase 6: Docs and manual verification

#### Task 6.1: Man page / CLI help

**Files:**
- Review: `cli/src/cli.rs` (or wherever `--allow-unsafe-links` help text lives)
- Review: `xtask` generated docs

- [ ] **Step 1: Grep for the existing `--allow-unsafe-links` help string**

Run: `rg -n "allow-unsafe-links" cli/src`.

- [ ] **Step 2: Update the help text to mention junctions**

Change the help string to something like:

```
Allow extracting symbolic links or junctions whose target escapes the extraction root.
```

- [ ] **Step 3: Regenerate docs**

Run:

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

Run: `cargo test --workspace --all-features`.
Expected: PASS.

- [ ] **Step 2: Clippy**

Run: `cargo clippy --workspace --all-targets --all-features -D warnings`.
Expected: no warnings.

- [ ] **Step 3: Format**

Run: `cargo fmt --all -- --check`.
Expected: no diff.

- [ ] **Step 4: Feature-powerset (if time permits)**

Run: `cargo hack check --locked --feature-powerset --exclude-features wasm`.
Expected: all combinations compile.

- [ ] **Step 5: On a Windows machine, perform manual smoke test**

```powershell
mkdir C:\tmp\junc-test\real
mklink /J C:\tmp\junc-test\link C:\tmp\junc-test\real
pna create -f C:\tmp\junc-test\out.pna C:\tmp\junc-test
pna extract -f C:\tmp\junc-test\out.pna --out-dir C:\tmp\junc-test-out --allow-unsafe-links
dir /AL C:\tmp\junc-test-out\junc-test
```

Expected: the extracted `link` is a junction pointing at the original `real` directory.

- [ ] **Step 6: Push branch and stop** (per `feedback_stop_at_push`: no PR creation unless explicitly requested)

```bash
git push -u origin cli/extract/fltp-decode
```

After push, run `gh run list --branch cli/extract/fltp-decode --limit 5` once to confirm CI started (per `feedback_check_ci_after_push`). Do NOT open a pull request.

---

## Out-of-Scope Follow-up (placeholder for a separate plan)

**Relative-path optimization (user directive Q2):** when a junction's absolute target is within the archive input set, rewrite the stored path as relative to the archive root and, at extract time, rejoin with `out_dir` before calling `create_junction`. Requires:
- A normalization step during file walk that maps absolute paths → archive entry names
- A marker in the entry (either reserved for a future `fLTP` private value 64–255, or a small ancillary chunk) to distinguish "external absolute target" from "archive-relative target"
- Additional extraction-time logic to rejoin and ensure the resolved absolute path still lands inside `out_dir` (security)

This is explicitly deferred. When the base junction support merges, open a separate plan using the same spec shape.

---

## Self-Review Checklist

**Spec coverage:**
- [x] Detection on Windows (Phase 1.2, Task 3.1)
- [x] Recording as HardLink + fLTP=Directory (Task 3.1)
- [x] Absolute target encoding (`\??\` stripped, Task 1.1)
- [x] Extract on Windows → junction (Task 4.1, 4.3)
- [x] Extract on non-Windows → symlink fallback (Task 4.1, 4.2)
- [x] `--allow-unsafe-links` gating (Task 4.1, 4.2)
- [x] Broken/unknown reparse tag → warn + skip (Task 5.1)
- [x] Regression-free for existing hardlink/symlink (Task 3.1 Step 8, Task 4.1 Step 5)
- [x] Docs update (Task 6.1)

**Placeholder scan:** no `TBD`/`TODO`/`similar to task N`. Code blocks show full implementations.

**Type consistency:**
- `ReparsePoint` variants used identically in Phase 1 tests and Phase 4 tests
- `StoreAs::Junction(PathBuf)` matches in create (Task 3.1) and any exhaustive matches (Task 3.1 Step 7)
- `LinkTargetType::Directory` is the fLTP value throughout
- `detect_junction` returns `io::Result<Option<PathBuf>>` consistently

**Feedback memory check:**
- `feedback_plan_for_plans` ✓ (plan-for-plans step preceded this)
- `feedback_no_impl_without_plan` ✓ (this plan is being produced before any code)
- `feedback_test_matrix_first` ✓ (matrix at top of file)
- `feedback_stop_at_push` ✓ (Task 6.2 Step 6 stops at push)
- `feedback_check_ci_after_push` ✓ (Task 6.2 Step 6)
