# Windows Junction Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Design spec:** [`docs/superpowers/specs/2026-04-19-windows-junction-support-design.md`](../specs/2026-04-19-windows-junction-support-design.md). Read it first for problem framing, goal, non-goals, architecture, interfaces, data flow, safety invariants, error-handling strategy, testing strategy, rejected alternatives, and deferred follow-ups. This plan is the step-by-step execution path for the agreed design.

**Tech stack (execution snapshot):** Rust 2024, MSRV 1.88. Existing `windows = "0.62.2"` dependency in `cli/Cargo.toml:67-73`; this plan adds four features (`Win32_Foundation`, `Win32_Security`, `Win32_System_Ioctl`, `Win32_System_IO`). No changes to `pna/Cargo.toml` or `libpna`.

---

## Scope reference

- **In scope:** see spec §2 (Goal) and §4 (Architecture). This plan executes that scope.
- **Out of scope / deferred:** see spec §3 (Non-goals) and §11 (Follow-ups).

## Prerequisites per Task

1. **Aegis consultation before code changes.** Per project `CLAUDE.md`, every task that touches source files must start by calling `aegis_compile_context` with `target_files` set to the files listed in the task header, `plan` set to the task's action, and `command` set to `"scaffold"` or `"refactor"` as appropriate. Follow returned guidelines. Report `compile_miss` if any guideline was missing.
2. **TDD order.** Each task writes the failing test first, runs it to confirm the failure, implements the minimum change, re-runs, then commits. This order is non-negotiable — `feedback_no_impl_without_plan` applies.
3. **Commit message.** Follow the project convention at `CLAUDE.md`'s emoji table. Do not add `Co-Authored-By` lines (global `CLAUDE.md`).

## Risks and safety invariants

See spec §7 (Safety invariants I1–I4) and §8 (Error handling). The plan's task steps enforce them at these anchors:

- **I1** (UTF-16 gate on junction target) — Task 1.1 (`parse_reparse_buffer` rejects invalid UTF-16).
- **I2** (junction extract must not mutate the external target) — Task 4.1 (early return after `restore_link_timestamps_no_follow`). Regression fence in Task 4.4.
- **I3** (junction classified before symlink on Windows) — Task 3.1.
- **I4** (`--allow-unsafe-links` required) — Task 4.1.

## Testing coverage

Spec §9 defines tests T1–T19. This plan lands them in the following files:

| Test set | File |
|---|---|
| T1–T5 (reparse parser + FFI round-trip) | `cli/src/utils/os/windows/fs/reparse.rs` tests module |
| T6–T7 (junction classification) | `cli/src/utils/os/windows/fs/junction.rs` tests module |
| T8–T10 (`PathnameEditor::edit_junction`) | `cli/src/command/core/path.rs` tests module |
| T11–T16, T19 (integration + security fence) | `cli/tests/cli/junction.rs` |
| T17–T18 (regression, existing) | `cli/tests/cli/hardlink.rs`, `cli/tests/cli/extract/hardlink.rs`, `cli/tests/cli/extract/symlink*.rs` |

## File touch points

| Action | Path |
|---|---|
| Create | `cli/src/utils/os/windows/fs/reparse.rs` |
| Create | `cli/src/utils/os/windows/fs/junction.rs` |
| Create | `cli/tests/cli/junction.rs` |
| Modify | `cli/Cargo.toml` (add four `windows` crate features) |
| Modify | `cli/src/utils/os/windows/fs.rs` (add `pub(crate) mod reparse;` and `pub(crate) mod junction;` next to the existing `pub(crate) mod owner;`; no other change) |
| Modify | `cli/src/command/core/path.rs` (extract shared helper, add `edit_junction`) |
| Modify | `cli/src/command/core.rs` (add `StoreAs::Junction`, classifier, create arm) |
| Modify | `cli/src/command/extract.rs` (junction branch + `create_junction_or_fallback` + `restore_link_timestamps_no_follow`) |
| Modify | `cli/tests/cli/main.rs` (register `mod junction;`) |

No changes to `lib/`, `pna/`, or the PNA specification repository.

---

## Task List

### Phase 1: CLI Windows reparse-point primitives

Goal: expose a minimal, testable set of wrappers over NTFS reparse points inside the CLI crate. Windows-only. No PNA logic here.

#### Task 1.1: Create module skeletons, Cargo feature bump, and reparse-buffer parser

**Files:**
- Create: `cli/src/utils/os/windows/fs/reparse.rs`
- Modify: `cli/src/utils/os/windows/fs.rs` (add `pub(crate) mod reparse;` submodule declaration; do **not** rename or move any existing item — `FileHandle`, `chmod`, `lchown`, `open_read_metadata`, and `pub(crate) mod owner;` all stay as-is)
- Modify: `cli/Cargo.toml` (extend `windows` features)

- [ ] **Step 1: Call `aegis_compile_context`**

Call `aegis_compile_context` with:
- `target_files: ["cli/src/utils/os/windows/fs/reparse.rs", "cli/src/utils/os/windows/fs.rs", "cli/Cargo.toml"]`
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
  "Win32_System_IO",
  "Win32_System_Ioctl",
  "Win32_System_Threading",
  "Win32_System_WindowsProgramming",
] }
```

- [ ] **Step 3: Create the module skeletons with only the parser tests (no parser implementation yet)**

Modify `cli/src/utils/os/windows/fs.rs` to add a `reparse` submodule declaration next to the existing `pub(crate) mod owner;` line at the top of the file. Do **not** yet declare `junction` — its file is created in Task 1.4, and a forward declaration without the file would fail to compile. The resulting prologue should look like:

```rust
pub(crate) mod owner;
pub(crate) mod reparse;

use super::security::{Sid, apply_security_info};
// ... existing use lines and body unchanged ...
```

Create `cli/src/utils/os/windows/fs/reparse.rs`:

```rust
//! Windows NTFS reparse-point primitives.
//!
//! This module is Windows-only. It is declared from
//! `cli/src/utils/os/windows/fs.rs`, which in turn is reached from
//! `cli/src/utils/os/windows.rs` — both ancestors are already gated on
//! `cfg(windows)` by `cli/src/utils/os.rs`, so no `#[cfg]` attribute on
//! individual items is required here.

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

    /// Build a REPARSE_DATA_BUFFER for a symbolic link pointing at `subst`.
    fn sample_symlink_buffer(subst: &str, is_relative: bool) -> Vec<u8> {
        // SymbolicLinkReparseBuffer layout:
        //   ReparseTag          u32 = 0xA000_000C
        //   ReparseDataLength   u16
        //   Reserved            u16 = 0
        //   SubstituteNameOffset u16
        //   SubstituteNameLength u16
        //   PrintNameOffset     u16
        //   PrintNameLength     u16
        //   Flags               u32 (bit 0 = SYMLINK_FLAG_RELATIVE)
        //   PathBuffer          [u16 UTF-16 LE, no null terminator]
        let subst_utf16: Vec<u8> = subst
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let print_utf16 = subst_utf16.clone();
        let mut path_buffer = subst_utf16.clone();
        path_buffer.extend(&print_utf16);

        let subst_offset: u16 = 0;
        let subst_len: u16 = subst_utf16.len() as u16;
        let print_offset: u16 = subst_len;
        let print_len: u16 = print_utf16.len() as u16;
        let flags: u32 = if is_relative { 0x1 } else { 0x0 };

        let mut buf = Vec::new();
        buf.extend(&0xA000_000Cu32.to_le_bytes()); // IO_REPARSE_TAG_SYMLINK
        let data_len: u16 = (12 + path_buffer.len()) as u16; // 4 offsets/lens + Flags + PathBuffer
        buf.extend(&data_len.to_le_bytes());
        buf.extend(&0u16.to_le_bytes()); // Reserved
        buf.extend(&subst_offset.to_le_bytes());
        buf.extend(&subst_len.to_le_bytes());
        buf.extend(&print_offset.to_le_bytes());
        buf.extend(&print_len.to_le_bytes());
        buf.extend(&flags.to_le_bytes());
        buf.extend(&path_buffer);
        buf
    }

    #[test]
    fn parses_absolute_symlink_and_strips_nt_prefix() {
        let buf = sample_symlink_buffer(r"\??\C:\target", false);
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(
            parsed,
            ReparsePoint::Symlink {
                target: PathBuf::from(r"C:\target"),
                is_relative: false,
            }
        );
    }

    #[test]
    fn parses_relative_symlink_without_stripping() {
        let buf = sample_symlink_buffer(r"..\target", true);
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(
            parsed,
            ReparsePoint::Symlink {
                target: PathBuf::from(r"..\target"),
                is_relative: true,
            }
        );
    }

    #[test]
    fn truncated_symlink_buffer_errors() {
        // Tag(4) + DataLength(2) + Reserved(2) + 4 offsets/lens(8) = 16 bytes.
        // Flags word would start at offset 16; truncate before it so the symlink
        // arm's length guard fires with InvalidData.
        let mut buf = vec![0u8; 16];
        buf[0..4].copy_from_slice(&0xA000_000Cu32.to_le_bytes());
        let err = parse_reparse_buffer(&buf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
```

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
git add cli/Cargo.toml cli/src/utils/os/windows/fs.rs cli/src/utils/os/windows/fs/reparse.rs
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

Also append a pure-Rust unit test for the `io_error_from_win32` helper so the HRESULT/Win32 round-trip is verified without needing Windows CI:

```rust
#[test]
fn io_error_from_win32_extracts_win32_code() {
    // HRESULT_FROM_WIN32(ERROR_NOT_A_REPARSE_POINT = 4390) == 0x80071126.
    let hr = windows::core::HRESULT(0x8007_1126u32 as i32);
    let err = super::io_error_from_win32(WinError::from(hr));
    assert_eq!(err.raw_os_error(), Some(4390));
}

#[test]
fn io_error_from_win32_passes_through_non_win32_hresults() {
    // E_FAIL (0x80004005) has facility FACILITY_NULL (0x0), not FACILITY_WIN32.
    let hr = windows::core::HRESULT(0x8000_4005u32 as i32);
    let err = super::io_error_from_win32(WinError::from(hr));
    assert_eq!(err.raw_os_error(), Some(0x8000_4005u32 as i32));
}
```

(Import `windows::core::Error as WinError` inside the tests module, or reference it via `super::WinError` if the helper re-exports it.)

`tempfile` is already in `cli/Cargo.toml` `[dev-dependencies]` (verify with `grep '^tempfile' cli/Cargo.toml`; if missing, add `tempfile = "3"`).

- [ ] **Step 3: Run to confirm failure**

Run: `cargo test -p portable-network-archive --lib utils::os::windows::fs::reparse::tests::read_reparse_point_on_junction --target x86_64-pc-windows-msvc`.
Expected: FAIL with `cannot find function read_reparse_point in module super`.

- [ ] **Step 4: Implement `read_reparse_point`**

Add to `cli/src/utils/os/windows/fs/reparse.rs` (below `parse_reparse_buffer`):

```rust
use std::{os::windows::ffi::OsStrExt, path::Path};

use windows::{
    core::{Error as WinError, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, GENERIC_READ, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{Ioctl::FSCTL_GET_REPARSE_POINT, IO::DeviceIoControl},
    },
};

/// Convert a [`windows::core::Error`] into an [`io::Error`] whose
/// [`raw_os_error()`](io::Error::raw_os_error) returns the canonical Win32
/// error code (e.g. `ERROR_NOT_A_REPARSE_POINT` = 4390) rather than the
/// HRESULT-encoded form that the `windows` crate uses internally.
///
/// Background: `windows::core::Error::from_win32()` wraps a Win32 error via
/// `HRESULT_FROM_WIN32(dwErr) = 0x80070000 | dwErr`, so for
/// `ERROR_NOT_A_REPARSE_POINT` (4390) the HRESULT payload is `0x80071126`
/// which, as an `i32`, is `-2147020506`. Passing that value through
/// `io::Error::from_raw_os_error` preserves the HRESULT bits, and downstream
/// comparisons like `err.raw_os_error() == Some(4390)` silently never match.
///
/// This helper detects HRESULTs whose facility is `FACILITY_WIN32` (0x7),
/// extracts the low 16 bits as the Win32 code, and passes anything else
/// through unchanged. Use this in preference to
/// `io::Error::from_raw_os_error(e.code().0)` everywhere the `windows` crate
/// surfaces a `Result<_, windows::core::Error>`.
fn io_error_from_win32(e: WinError) -> io::Error {
    let hr = e.code().0 as u32;
    let facility = (hr >> 16) & 0x1FFF;
    let raw = if facility == 0x0007 {
        (hr & 0xFFFF) as i32
    } else {
        e.code().0
    };
    io::Error::from_raw_os_error(raw)
}

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
    .map_err(io_error_from_win32)?;

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
    ioctl_result.map_err(io_error_from_win32)?;
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
        let err = io_error_from_win32(e);
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
        let err = io_error_from_win32(e);
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
- Modify: `cli/src/utils/os/windows/fs.rs` (add `pub(crate) mod junction;` next to the `pub(crate) mod reparse;` declaration from Task 1.1)

- [ ] **Step 1: Call `aegis_compile_context`**

`target_files: ["cli/src/utils/os/windows/fs/junction.rs", "cli/src/utils/os/windows/fs.rs"]`, `plan: "Add detect_junction helper that maps ERROR_NOT_A_REPARSE_POINT to Ok(None)"`, `command: "scaffold"`.

- [ ] **Step 2: Declare the submodule and write the test skeleton**

First, add `pub(crate) mod junction;` to `cli/src/utils/os/windows/fs.rs` right below the existing `pub(crate) mod reparse;` declaration added in Task 1.1.

Then create `cli/src/utils/os/windows/fs/junction.rs`:

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
git add cli/src/utils/os/windows/fs.rs cli/src/utils/os/windows/fs/junction.rs
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

        // SAFETY: The default `restore_metadata()` call at the end of this
        // function would apply chmod/chown/ACL/xattr/fflags to the junction
        // via follow-link syscalls, which would mutate the EXTERNAL
        // directory the junction points at (outside the extraction root).
        // For the MVP we bypass the full metadata restore and only apply
        // no-follow timestamp restoration. See
        // `restore_link_timestamps_no_follow` for the follow-up plan that
        // properly restores junction-owned metadata.
        restore_link_timestamps_no_follow(&path, item.metadata(), keep_options)?;
        return Ok(());
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

Also add the no-follow timestamp helper near the bottom of `extract.rs` (next to the other helpers):

```rust
/// Restore only the timestamps of a junction or fallback-symlink entry,
/// without following the link.
///
/// # Why not the full `restore_metadata()` path?
///
/// `DataKind::HardLink + fLTP=Directory` encodes a Windows junction (or a
/// fallback symlink on non-Windows). The default `restore_metadata()` uses
/// `chmod`, `chown`, `set_facl(follow_links=true)`, `setxattr`, `set_flags`,
/// and macOS `copyfile` — every one of those follows links. If we let them
/// run against the junction path, they would mutate the **external**
/// directory the junction points at (outside the extraction root), which is
/// a security hole.
///
/// For the MVP we bypass the full metadata path for junction entries and
/// apply only timestamps through `filetime::set_symlink_file_times`, which
/// opens the reparse point itself with `FILE_FLAG_OPEN_REPARSE_POINT` on
/// Windows and uses `utimensat(AT_SYMLINK_NOFOLLOW)` / `lutimes` on Unix.
///
/// # TODO: junction-aware no-follow metadata (deferred follow-up)
///
/// A full implementation should restore mode/owner/ACL/xattr/fflags on the
/// junction itself using no-follow APIs:
/// - Unix: `lchmod` (BSD), `lchown`, `lsetxattr`, `lremovexattr`.
/// - Linux: mode-on-symlink is not supported by the kernel; either skip
///   silently or gate behind `#[cfg(target_os = "linux")]` with a `warn!`.
/// - Windows: open the reparse point via `FILE_FLAG_OPEN_REPARSE_POINT` and
///   apply ACL/security info with `SetSecurityInfo` on that handle; mode is
///   expressed via the Windows security descriptor, not `chmod`.
/// - ACL restoration must pass `follow_links = false` into `restore_acls`
///   (currently that path silently returns on Windows non-symlink entries,
///   which would need generalization).
///
/// When implemented, replace this helper with a junction-aware branch of
/// `restore_metadata` that keeps the safety invariant but preserves all
/// attributes.
fn restore_link_timestamps_no_follow<T>(
    path: &Path,
    metadata: &pna::Metadata<T>,
    keep_options: &KeepOptions,
) -> io::Result<()>
where
    T: AsRef<[u8]>,
{
    #[cfg(not(target_family = "wasm"))]
    {
        // Reuse the existing `restore_path_timestamps` helper if and only if
        // it already uses `filetime::set_symlink_file_times`. Inspect
        // `restore_path_timestamps` at implementation time: if it uses the
        // follow-link `set_file_times`, introduce a sibling
        // `restore_path_timestamps_no_follow` that calls
        // `filetime::set_symlink_file_times` instead. Do NOT reuse the
        // follow-link version for junction entries.
        utils::fs::restore_path_timestamps_no_follow(path, metadata, keep_options)?;
    }
    #[cfg(target_family = "wasm")]
    {
        // No no-follow timestamp API on WASM target; skip silently.
        let _ = (path, metadata, keep_options);
    }
    Ok(())
}
```

If `utils::fs::restore_path_timestamps_no_follow` does not yet exist, add it alongside the existing `restore_path_timestamps` in the same module. The implementation difference is one line — `set_file_times` → `set_symlink_file_times` — and the signature is identical.

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

#### Task 4.4: Security regression — `--keep-permission` must not mutate the external junction target

**Files:**
- Modify: `cli/tests/cli/junction.rs`

This test locks in the MVP safety contract: even when the user passes
attribute-restoration flags that would normally invoke `chmod` / `chown`
/ `set_facl`, the **external directory** that the junction points at must
not be touched. It is the regression fence for the `critical` finding
raised by Codex's adversarial review.

- [ ] **Step 1: Write the failing test**

Append to `cli/tests/cli/junction.rs`:

```rust
/// Precondition: archive with a HardLink+fLTP=Directory entry pointing at
/// an existing external directory whose owner/mode the user has already
/// set. The archive's recorded metadata intentionally differs from the
/// target's current metadata.
/// Action: extract with `--allow-unsafe-links --keep-permission`
/// (`--same-owner` / `--keep-acl` on Unix; equivalent flags elsewhere).
/// Expectation: the junction or fallback symlink is created, but the
/// external target directory's mode/owner/ACL remain untouched. This pins
/// the "junction extract does not mutate its external target" invariant.
#[test]
fn extract_junction_does_not_mutate_external_target() {
    use std::os::unix::fs::PermissionsExt; // Unix-only assertion; Windows uses its own block below.

    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("external_target");
    std::fs::create_dir(&target).unwrap();

    // Pre-set a recognizable mode on the external target.
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(&target).unwrap().permissions();
        perms.set_mode(0o700);
        std::fs::set_permissions(&target, perms).unwrap();
    }
    let baseline_mode = std::fs::metadata(&target).unwrap().permissions();

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
        .arg("--keep-permission")
        .status()
        .unwrap();
    assert!(status.success());

    // The link must exist.
    let link = out_dir.join("link_dir");
    let link_meta = std::fs::symlink_metadata(&link).unwrap();
    assert!(link_meta.file_type().is_symlink() || {
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            link_meta.file_type().is_symlink_dir()
        }
        #[cfg(not(windows))]
        {
            false
        }
    });

    // The external target's metadata must be byte-for-byte unchanged.
    let after_mode = std::fs::metadata(&target).unwrap().permissions();
    assert_eq!(
        baseline_mode, after_mode,
        "extract --keep-permission must NOT mutate the external junction target"
    );
}
```

- [ ] **Step 2: Run to confirm failure, then pass**

Run before the Task 4.1 fix is in place: this test fails because
`restore_metadata` follows the junction and mutates `external_target`'s
mode.

Run after Task 4.1's early-return + `restore_link_timestamps_no_follow`
wiring is in place: this test passes. Expected PASS on both Unix
(fallback symlink) and Windows (real junction).

- [ ] **Step 3: Commit**

```bash
git add cli/tests/cli/junction.rs
git commit -m ":white_check_mark: Pin junction extract does-not-mutate-target invariant"
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

## Out-of-scope follow-ups

Captured in spec §11. In short: relative-path CREATE optimization, junction-aware no-follow metadata (Option C), and public reparse-point API.

---

## Self-Review Checklist

**Spec coverage (WHAT is implemented and WHERE):**
- [x] Junction detection on Windows — Task 1.4 (primitive), Task 3.1 (classifier)
- [x] CREATE: record junction as HardLink + fLTP=Directory — Task 3.1
- [x] Reparse buffer parser with `\??\` strip — Task 1.1
- [x] EXTRACT Windows → real junction — Task 4.1, 4.3
- [x] EXTRACT non-Windows → symlink fallback — Task 4.1, 4.2
- [x] EXTRACT accepts absolute and relative targets — Task 4.1
- [x] `--allow-unsafe-links` gate — Task 4.1, 4.2
- [x] Unknown reparse tag → debug log + fall-through — Task 3.1 Step 5, Task 5.1
- [x] Regression-free for existing hardlink/symlink — Task 3.1 Step 9, Task 4.1 Step 5, Task 5.2
- [x] Help-text + regenerated docs — Task 6.1
- [x] `PathnameEditor::edit_junction` with shared helper — Task 2.1
- [x] No `pna` / `libpna` modifications — by construction
- [x] Safety invariant I2 (junction extract does not mutate external target) — Task 4.1 early return + `restore_link_timestamps_no_follow`; regression fence in Task 4.4

**Placeholder scan:** no `TBD` / `TODO` inside task steps. A single `TODO: junction-aware no-follow metadata (deferred follow-up)` block inside `restore_link_timestamps_no_follow` is intentional and aligned with spec §11.

**Type consistency across tasks:**
- `ReparsePoint` enum — Task 1.1 (parser) and Task 1.2 (FFI wrapper) use identical variants.
- `StoreAs::Junction(PathBuf)` — introduced in Task 3.1 Step 4, consumed in Task 3.1 Step 7.
- `LinkTargetType::Directory` is the fLTP throughout.
- `detect_junction` signature `fn(&Path) -> io::Result<Option<PathBuf>>` matches at definition (Task 1.4) and at every call site (Task 3.1).
- `PathnameEditor::edit_junction` returns `EntryReference`, consumed in Task 4.1 via `.as_str()`.
- `create_junction_or_fallback(&Path, &str)` signature matches both call site (Task 4.1) and helper definition.
- `restore_link_timestamps_no_follow(&Path, &pna::Metadata<T>, &KeepOptions)` is defined and called only in Task 4.1; it is the single chokepoint the Option C follow-up will extend.

**Feedback memory check:**
- `feedback_plan_for_plans` ✓ (plan + spec both adversarially grilled before committing).
- `feedback_no_impl_without_plan` ✓ (spec and plan exist before any production code).
- `feedback_test_matrix_first` ✓ (T1–T19 matrix in spec §9, file anchors above).
- `feedback_stop_at_push` ✓ (Task 6.2 Step 6 stops at push).
- `feedback_check_ci_after_push` ✓ (Task 6.2 Step 6).
- `feedback_subagent_opus_only` ✓ (subagents for implementation/review dispatched with `model: "opus"`).
- `feedback_safe_bulk_edit` ✓ (no `replace_all`; all edits are targeted).
- `feedback_confirm_irreversible` ✓ (only the final `git push` is non-local).
- `feedback_verify_claims` ✓ (`preserve_root` API and `windows` crate features were verified via source / `cargo check` before committing this plan).
