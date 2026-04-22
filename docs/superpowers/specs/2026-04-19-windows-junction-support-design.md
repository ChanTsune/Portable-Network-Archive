# Windows Junction Support — Design

## 1. Problem

Windows NTFS **junctions** (reparse points tagged `IO_REPARSE_TAG_MOUNT_POINT`) link one directory onto another. The PNA archive tool (`pna`) currently cannot preserve them:

- Create-side classification treats every reparse point as a plain symbolic link. On Windows, a junction and a directory symlink both return `true` from `fs::symlink_metadata().file_type().is_symlink()`, so the distinction is lost and the entry is recorded as `DataKind::SymbolicLink` with whatever target string `fs::read_link` returns (often truncated or garbled for junction reparse buffers that include the `\??\` NT-object prefix).
- Extract-side creation uses `std::os::windows::fs::symlink_dir`, which creates a real symbolic link, **not** a junction. Round-tripping a junction through `pna` therefore silently downgrades the link flavor.
- Reference tooling (`libarchive` / `bsdtar`) also has no junction support (upstream issue #2527, open at the time of writing). PNA is free to ship junction round-tripping as a differentiator for Windows archive workflows.

## 2. Goal

Detect Windows junctions during archive creation and restore them during extraction, using the existing `HardLink + fLTP=Directory` encoding with an external target path stored as entry data. A round trip through `pna create` + `pna extract` must:

- produce a real junction on Windows (same reparse tag, same target);
- produce a symbolic link on non-Windows (per the PNA specification's MAY clause at `chunk_specifications/index.md:332-336`);
- leave every existing hardlink and symlink test unaffected;
- avoid any change to `libpna` or the `pna` wrapper crate — platform-specific Windows FFI must stay in `cli`.

## 3. Non-goals

The following are **explicitly out of scope** for this design. Each has a clear reason to defer.

- **Relative-path optimization on create.** When a junction's absolute target happens to point inside the archive's own input set, it could be rewritten as a relative path so the archive stays self-contained. Deferred to a separate plan; the extract side of this spec already accepts both absolute and relative stored targets, so the create-time optimization is a drop-in.
- **Other reparse tags.** App Execution Aliases (`IO_REPARSE_TAG_APPEXECLINK`), OneDrive placeholders (`IO_REPARSE_TAG_CLOUD_*`), dedup reparse points, and others are classified as `ReparsePoint::Other(tag)` and fall through to existing symlink handling. No ergonomic support is added for them.
- **PNA specification document changes.** The existing spec text already says "directory hard link = junction"; this design is the implementation that anchors the encoding.
- **UNC junction targets.** The Windows kernel forbids UNC targets for `IO_REPARSE_TAG_MOUNT_POINT` on create. The extract-side parser must not panic on them, but no round-trip support is promised.
- **Junction-aware no-follow metadata restoration (mode / owner / ACL / xattrs / fflags / macOS AppleDouble).** The MVP restores **only** no-follow timestamps for junction entries. The full no-follow attribute path needs `lchmod`, `lchown`, `lsetxattr`, `SetSecurityInfo` on reparse handles, and a generalization of `restore_acls` — large enough to warrant its own plan. Captured as a follow-up in §11.

## 4. Architecture

### Layer discipline

The codebase already enforces a three-tier layering: `libpna` ⊂ `pna` ⊂ `cli`. `libpna` handles chunk encoding and never touches the filesystem. `pna` adds cross-platform filesystem helpers but must remain **platform-dependency-free** so the crate can be depended on by portable consumers. All Windows-specific code therefore lives in `cli`.

| Layer | Change in this design |
|---|---|
| `libpna` | none — the `HardLink + fLTP=Directory` byte round trip is already guaranteed by the test `builder_hardlink_with_link_target_type_directory` at `lib/src/entry/builder.rs:783, :798`. |
| `pna` | none — no new platform-specific dependency introduced. |
| `cli/src/utils/os/windows/fs/reparse.rs` (new) | `ReparsePoint` enum, `parse_reparse_buffer`, `read_reparse_point`, `create_junction`, and the private `io_error_from_win32` translation helper. |
| `cli/src/utils/os/windows/fs/junction.rs` (new) | `detect_junction` — the classifier consumed by the create pipeline. |
| `cli/src/utils/os/windows/fs.rs` (modified) | adds `pub(crate) mod reparse;` and `pub(crate) mod junction;` alongside the existing `pub(crate) mod owner;`. Existing items (`FileHandle`, `chmod`, `lchown`, `open_read_metadata`) are untouched. |
| `cli/src/command/core/path.rs` | adds `PathnameEditor::edit_junction`, which delegates to a new private helper shared with `edit_symlink`. |
| `cli/src/command/core.rs` | adds `StoreAs::Junction(PathBuf)`, a classifier that runs *before* symlink classification on Windows, and a `create_entry` arm that emits the junction as `HardLink + fLTP=Directory + absolute target`. |
| `cli/src/command/extract.rs` | inside the `DataKind::HardLink` arm, branches on `fLTP == Some(Directory)` and takes a dedicated junction path: `edit_junction → create_junction_or_fallback → restore_link_timestamps_no_follow → early return`. |

### Encoding

The archive representation is `DataKind::HardLink` with `fLTP = LinkTargetType::Directory` and the junction's **external** target path (absolute, `\??\` prefix stripped, UTF-8) stored as the entry's byte payload. The `libpna` byte format does not change. The semantic reinterpretation is CLI-local: hardlink entries with `fLTP=Directory` mean "Windows junction or its non-Windows symlink fallback"; entries with `fLTP=File` or `None` continue to mean a normal hardlink.

### Non-Windows fallback

On non-Windows extraction, the junction entry is materialized as a symbolic link whose target is the stored string verbatim. This follows the PNA specification's MAY clause. The result is a dangling symlink in the common case (a Windows absolute path makes no sense on Unix), but it preserves the archive's intent in a form the foreign filesystem can represent. Users who want faithful Windows behavior extract on Windows.

## 5. Interfaces

### `cli/src/utils/os/windows/fs/reparse.rs` (Windows-only)

```rust
pub enum ReparsePoint {
    /// IO_REPARSE_TAG_MOUNT_POINT. Target is absolute by kernel invariant.
    Junction(PathBuf),
    /// IO_REPARSE_TAG_SYMLINK. `is_relative` reflects SYMLINK_FLAG_RELATIVE.
    Symlink { target: PathBuf, is_relative: bool },
    /// Any other reparse tag (cloud placeholders, AppExecLink, etc.).
    Other(u32),
}

pub(crate) fn read_reparse_point(path: &Path) -> io::Result<ReparsePoint>;
pub(crate) fn create_junction(link: &Path, target: &Path) -> io::Result<()>;

// Private, but unit-testable cross-platform via its byte-in byte-out signature.
fn parse_reparse_buffer(buf: &[u8]) -> io::Result<ReparsePoint>;
// Private. Translates windows::core::Error into io::Error with a Win32-code raw_os_error.
fn io_error_from_win32(e: windows::core::Error) -> io::Error;
```

The helpers are `pub(crate)`-visible; none of them leak outside the CLI.

### `cli/src/utils/os/windows/fs/junction.rs` (Windows-only)

```rust
pub fn detect_junction(path: &Path) -> io::Result<Option<PathBuf>>;
```

Classification rules:

- `read_reparse_point` returns `Junction(t)` → `Ok(Some(t))`.
- `read_reparse_point` returns `Symlink {..}` or `Other(_)` → `Ok(None)`. The caller falls through to existing symlink classification.
- `read_reparse_point` returns `Err(e)` with `e.raw_os_error() == Some(4390)` (`ERROR_NOT_A_REPARSE_POINT`) → `Ok(None)`. A regular directory is not a junction.
- Any other error is propagated.

### `cli/src/command/core.rs::StoreAs`

```rust
pub(crate) enum StoreAs {
    File,
    Dir,
    Symlink(LinkTargetType),
    Hardlink(PathBuf),
    Junction(PathBuf),   // NEW, produced only on Windows, declared unconditionally
}
```

The `Junction` variant is declared on every platform so that `match` sites remain exhaustive across `cfg`. Non-Windows `classify_junction` always returns `Ok(None)`, so the variant is unreachable there.

### `cli/src/command/core/path.rs::PathnameEditor`

```rust
impl PathnameEditor {
    pub(crate) fn edit_symlink(&self, target: &Path) -> EntryReference;   // existing
    pub(crate) fn edit_junction(&self, target: &Path) -> EntryReference;  // NEW
    // Private helper that both methods delegate to.
    fn transform_link_target_preserving_root(&self, target: &Path) -> EntryReference;
}
```

`edit_junction` exists as its own method — despite currently sharing semantics with `edit_symlink` — so that any future divergence (for example, restricting junction targets to absolute paths, or refusing UNC) only affects callers that actually traverse junctions. Both methods apply user-supplied substitutions (`-s` / `--transform`) but skip `--strip-components` and the bsdtar "strip leading slash" rule, matching bsdtar's symlink-target handling.

## 6. Data flow

### Create

```
walk(input set)
  │
  ▼ is_symlink (includes junctions on Windows)?
  │
  ├─ yes ──▶ classify_junction:
  │           ├─ Some(absolute_target) ──▶ StoreAs::Junction(absolute_target)
  │           └─ None                  ──▶ classify_symlink ──▶ StoreAs::Symlink(ltp)
  │
  └─ no  ──▶ StoreAs::File / Dir / Hardlink

create_entry(StoreAs::Junction(target)):
    EntryBuilder::new_hard_link(
        entry_name,
        pathname_editor.edit_junction(target),
    )
    .link_target_type(LinkTargetType::Directory)
    .apply_metadata(...)
    .build()
```

`edit_junction` is the canonical create-side entry point for junction targets. It applies user-specified `-s` / `--transform` substitutions on the same footing as `edit_symlink` does for symbolic-link targets, then builds an `EntryReference` via the shared helper. Invariant I1 (§7) guarantees the target is valid UTF-8 before it reaches this point, so the helper's internal `to_string_lossy` is effectively lossless for real inputs.

### Extract

```
extract_link_entry dispatches on DataKind:

  DataKind::SymbolicLink  ──▶ (unchanged)
    edit_symlink → symlink_with_type → restore_metadata

  DataKind::HardLink:
    is_directory_link = metadata().link_target_type() == Some(Directory)
    ├─ yes — JUNCTION PATH:
    │     edit_junction(stored_target_str)
    │     if !allow_unsafe_links: warn + return Ok(())
    │     create_junction_or_fallback(link_path, transformed_target):
    │        Windows   : reparse::create_junction (absolute after canonicalize)
    │        non-Windows: utils::fs::symlink (target stored verbatim)
    │     restore_link_timestamps_no_follow(link_path, metadata, keep_options)
    │     EARLY RETURN  ◀── CRITICAL: skips default restore_metadata
    │
    └─ no — REGULAR HARDLINK PATH: (unchanged)
         edit_hardlink → fs::hard_link → restore_metadata
```

The early return is the single point that enforces the external-target safety invariant (I2, §7). Nothing past it may touch the junction path with a follow-link syscall.

## 7. Safety invariants

| ID | Invariant | Enforcement point |
|---|---|---|
| I1 | Every junction target that reaches `EntryReference` is valid UTF-8. | `parse_reparse_buffer` calls `String::from_utf16`, which rejects invalid UTF-16 (including unpaired surrogates) with `ErrorKind::InvalidData`. |
| I2 | Extract must never apply a follow-link syscall to a junction entry's path. | The `is_directory_link` branch of the `DataKind::HardLink` arm returns before `restore_metadata` runs, and `restore_link_timestamps_no_follow` uses `filetime::set_symlink_file_times` internally. Regression-fenced by test T19. |
| I3 | On Windows, junction classification runs **before** symlink classification. | `classify_junction(path)?` is invoked before `detect_symlink_target_type` inside the `is_symlink` branch of create-time item collection. |
| I4 | Junction targets are always treated as unsafe external references. | `--allow-unsafe-links` is required to extract a junction entry; without it the entry is warned and skipped. |

I2 is load-bearing. Without it, an attacker-crafted archive whose junction entry points at `C:\Windows\System32` can mutate that directory's mode, ownership, or ACLs during `pna extract --keep-permission`.

## 8. Error handling

### Windows error translation

The `windows` crate returns failures as `windows::core::Error`, whose payload is an HRESULT. Win32 error codes are HRESULT-encoded via `HRESULT_FROM_WIN32(code) = 0x80070000 | code`. For example, `ERROR_NOT_A_REPARSE_POINT` (4390) becomes the HRESULT `0x80071126`, which is `-2147020506` when interpreted as `i32`. Passing that value through `io::Error::from_raw_os_error` stores the HRESULT bits, and a downstream `err.raw_os_error() == Some(4390)` comparison never matches.

`io_error_from_win32` inspects the HRESULT facility. When FACILITY_WIN32 (0x7) is set, it extracts the low 16 bits as the Win32 code and stores **that** as `raw_os_error`. Non-Win32 HRESULTs are passed through verbatim. Every `map_err` in `read_reparse_point` and `create_junction` routes through this helper, so `detect_junction`'s comparison against 4390 is in terms of the Microsoft-documented canonical form.

### Non-UTF-8 defensive handling

Junction targets reach the create path through `PathnameEditor::edit_junction`, which applies user-specified `-s` / `--transform` substitutions and then delegates to the shared `transform_link_target_preserving_root` helper that `edit_symlink` also uses. That helper converts via `to_string_lossy` internally, but invariant I1 guarantees the target is valid UTF-8 by the time it reaches `edit_junction`, so the conversion is effectively lossless for real inputs. If a future change were to let a non-UTF-8 `PathBuf` through, the lossy fallback degrades gracefully (replacement characters) rather than panicking — the loud-failure surface for an I1 violation is `parse_reparse_buffer`'s `String::from_utf16` gate upstream, not the create path.

### Metadata restoration (MVP Option A)

Junction entries bypass `restore_metadata()` entirely. `restore_link_timestamps_no_follow(link, metadata, keep_options)` applies **only** timestamps through `filetime::set_symlink_file_times`, which uses `FILE_FLAG_OPEN_REPARSE_POINT` on Windows and `utimensat(AT_SYMLINK_NOFOLLOW)` or `lutimes` on Unix. Mode, ownership, ACLs, xattrs, fflags, and macOS AppleDouble data are **skipped**. The skip is safe (it never mutates the external target) but lossy (junction-owned attributes are not preserved).

### Reparse-inspection errors

Any error other than `ERROR_NOT_A_REPARSE_POINT` from `detect_junction` is logged at `debug!` level and swallowed to `Ok(None)` by `classify_junction`. The entry falls through to existing symlink classification. This is quiet by design: a `warn!` for every permission-denied reparse probe during a recursive walk would be noisy and provide no actionable signal, while the fallback still records the entry with best-effort semantics.

## 9. Testing strategy

| # | Scenario | Platform | Kind |
|---|---|---|---|
| T1 | `parse_reparse_buffer` extracts junction `SubstituteName` and strips `\??\` | Windows (unit, pure byte parser) | unit |
| T2 | `parse_reparse_buffer` rejects truncated buffers with `InvalidData` | Windows | unit |
| T3 | `parse_reparse_buffer` reports unknown tags as `ReparsePoint::Other` | Windows | unit |
| T4 | `read_reparse_point` round-trips a real `mklink /J` junction | Windows | unit |
| T5 | `create_junction` + `read_reparse_point` round trip | Windows | unit |
| T6 | `detect_junction` returns `Some(target)` for a real junction | Windows | unit |
| T7 | `detect_junction` returns `Ok(None)` for a regular directory (maps `ERROR_NOT_A_REPARSE_POINT`) | Windows | unit |
| T8 | `PathnameEditor::edit_junction` preserves `C:\abs\target` | cross-platform | unit |
| T9 | `PathnameEditor::edit_junction` preserves `/abs/target` | cross-platform | unit |
| T10 | `PathnameEditor::edit_junction` does **not** apply `--strip-components` | cross-platform | unit |
| T11 | Create records a junction as `HardLink + fLTP=Directory + absolute` | Windows | integration |
| T12 | Full round trip: `pna create` → `pna extract` → real junction | Windows | integration |
| T13 | Extract `HardLink + fLTP=Directory` with absolute target on non-Windows → symlink | Unix | integration |
| T14 | Extract with relative target on Windows → canonicalized, created as junction | Windows | integration |
| T15 | Extract with relative target on non-Windows → symlink with relative target | Unix | integration |
| T16 | Extract without `--allow-unsafe-links` warns and skips | cross-platform | integration |
| T17 | Existing hardlink test suite regresses cleanly | cross-platform | existing |
| T18 | Existing symlink test suite regresses cleanly | cross-platform | existing |
| **T19** | Extract with `--keep-permission` does **not** mutate the external junction target (I2 regression fence) | cross-platform | integration |

T19 is the security fence. It pre-sets a recognizable mode (or owner, depending on platform) on an external directory, runs extract with `--allow-unsafe-links --keep-permission` against a fixture that points at that directory, and asserts the pre-set mode is byte-for-byte unchanged afterwards. Any change that re-opens the follow-link metadata path for junction entries will break this test.

## 10. Rejected alternatives

### Encoding

- **Case B'** (SymbolicLink + a new `jUNC` ancillary chunk). Rejected: the PNA specification already says "directory hard link = junction" (`chunk_specifications/index.md:332-336`). Introducing a parallel encoding splits the semantics across two places; this design anchors the existing spec claim instead.
- **Case C'** (new `DataKind::Junction`). Rejected: the existing encoding round-trips without format changes, and adding a new `DataKind` variant requires spec coordination and breaks old readers noisier than necessary.

### Module placement

- `pna/src/fs/reparse.rs`. Rejected: the `pna` crate is intentionally platform-dependency-free so portable consumers can depend on it without dragging Windows bindings. The Windows FFI lives in the CLI crate only.
- `cli/src/utils/os/windows/fs/mod.rs` (a new directory-style module). Rejected: Rust rejects a module having both `mod.rs` and a sibling `foo.rs` file, and the existing `cli/src/utils/os/windows/fs.rs` already hosts `FileHandle`, `chmod`, `lchown`, `open_read_metadata`, and `pub(crate) mod owner;`. Submodule declarations for `reparse` and `junction` are added inside the existing file.

### Metadata safety

- **Option B** (add an `is_link_entry` helper and extend existing `data_kind == SymbolicLink` guards). Rejected: only the mode and ACL code paths currently use that guard. Timestamps, xattrs, file flags, and macOS AppleDouble would still follow the junction. Option B therefore closes only part of the hole and creates a false sense of safety.
- **Option C** (full junction-aware no-follow API across every attribute). Not rejected outright; promoted to a follow-up plan captured in §11. MVP scope favors option A.

### Path editing

- Call `edit_symlink` directly for junction targets. Rejected — even though the current semantics are identical, giving junction its own method stops every call site from needing an `if-junction-else-symlink` conditional the day any semantics do diverge (e.g., a future decision to require absolute targets or reject UNC). The private shared helper keeps the duplication to one line.

### Error representation

- Match the HRESULT form directly (`err.raw_os_error() == Some(-2147020506)`). Rejected — the Microsoft-canonical `ERROR_NOT_A_REPARSE_POINT = 4390` is what documentation, log output, and other tooling use. Translating to that form inside `io_error_from_win32` keeps downstream call sites readable.

## 11. Follow-ups (deferred plans)

- **Create-side relative-path optimization.** When a junction's absolute target lives inside the archive input set, rewrite the stored path as a relative one. The extract side already handles both forms. Requires a mapping step in the walker and a rule for distinguishing "internal relative" from "external relative" entries.
- **Option C — junction-aware no-follow metadata.** Implement `lchmod` / `lchown` / `lsetxattr` / `lremovexattr` on Unix, open reparse points with `FILE_FLAG_OPEN_REPARSE_POINT` and apply security descriptors via `SetSecurityInfo` on Windows, generalize `restore_acls` so `follow_links = false` works on the Windows non-symlink path. Must keep T19 passing.
- **Publish reparse-point primitives.** If a downstream consumer wants to reuse `ReparsePoint` / `read_reparse_point` / `create_junction`, promote them into `pna` behind a `windows-fs` feature. The current design leaves them CLI-private because `pna` cannot carry platform dependencies.

## 12. Glossary

- **Junction** — an NTFS reparse point with tag `IO_REPARSE_TAG_MOUNT_POINT`. Non-privileged to create. Target must be a local absolute path; UNC targets are rejected by the kernel.
- **fLTP** — the `link_target_type` chunk bundled with link entries in PNA. Values: `File`, `Directory`, `Unknown`. This design uses `Directory` to mark junction-or-fallback-symlink hardlink entries.
- **`\??\` prefix** — the NT object namespace prefix present in reparse buffers' `SubstituteName` field. Stripped before storing the target.
- **Follow-link syscall** — a filesystem syscall that traverses symbolic links / junctions before acting (e.g., `chmod`, `chown`, `setxattr` without `AT_SYMLINK_NOFOLLOW`, `SetSecurityInfo` without a reparse-opened handle). Unsafe for junction entries because it mutates the external target.
