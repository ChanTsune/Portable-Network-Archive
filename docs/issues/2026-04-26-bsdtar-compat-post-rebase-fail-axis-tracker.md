# bsdtar-compat Post-Rebase Fail Axis Tracker

## Context

After rebase of `ci/bsdtar-compat-labels` onto main (Stage 1 completion), bsdtar-compat differential testing produced the following failures. These are out of scope for Stage 1 per the design — see `docs/plans/2026-04-26-Stage1-rebase-design.md` (option C, Stage 1 acceptance criteria).

This file replaces the originally-planned GitHub issue per user direction (md file artifact instead of `gh issue create`).

## Stage 1 final CI run

- Run ID: `24956230264`
- HEAD: `a460c18d` (Stage 1 + Cargo.lock fix)
- Workflow: `bsdtar compatibility`
- Conclusion: `failure` (overall, due to Windows job)

| Job | Conclusion | Notes |
|---|---|---|
| `Verify bsdtar_test passes with bsdtar (ubuntu)` | success | reference baseline |
| `Verify bsdtar_test passes with bsdtar (macos)` | success | reference baseline |
| `Verify bsdtar_test passes with bsdtar (windows)` | success | reference baseline |
| `Test pna compat bsdtar with bsdtar_test (ubuntu)` | success | PNA passes bsdtar_test on Linux ✅ |
| `Test pna compat bsdtar with bsdtar_test (macos)` | success | PNA passes bsdtar_test on macOS ✅ |
| `Test pna compat bsdtar with bsdtar_test (windows)` | **failure** | xtask build failure (see below) |

## Failure detail: Windows xtask build

Windows runner cannot build `xtask` due to unconditional `MetadataExt` import:

```
error[E0433]: cannot find `unix` in `os`
   --> xtask\src\bsdtar_compat.rs:3:14
error[E0432]: unresolved import `std::os::unix`
error[E0599]: no method named `mode` found for struct `std::fs::Metadata` (lines 612, 617)
error[E0599]: no method named `mtime` found for struct `std::fs::Metadata` (line 618)
error: could not compile `xtask` (bin "xtask") due to 5 previous errors
```

### Root cause

`xtask/src/bsdtar_compat.rs:3` uses `use std::os::unix::fs::{self as unix_fs, MetadataExt};` unconditionally. This module does not exist on Windows, so the entire crate fails to compile on the Windows runner.

### Resolution timeline

- **Stage 1**: Out of scope by design (option C). Windows compatibility was deferred.
- **Stage 2**: Will add `#[cfg(unix)]` sealing to `xtask/src/main.rs` so `bsdtar_compat` module is excluded on Windows targets, allowing `cargo build -p xtask` to succeed on Windows without the `bsdtar-compat` subcommand. See `docs/plans/2026-04-26-Stage2-fs-snapshot-extension-design.md` "Architecture > `#[cfg(unix)]` sealing" and the implementation plan `docs/plans/2026-04-26-Stage2-fs-snapshot-extension-implementation-plan.md` Task 2.

### Verification after Stage 2

After Stage 2 lands, this Windows failure should clear. The expected post-Stage-2 outcome:

- `Test pna compat bsdtar with bsdtar_test (windows)`: success (xtask builds, no `bsdtar-compat` subcommand exposed on Windows; `pna compat bsdtar` itself runs the bsdtar_test suite)

If Windows still fails after Stage 2, file a per-axis issue at that point.

## Other observations

- **Stage 1 Cargo.lock fix**: Initial Stage 1 push had Cargo.lock conflicts taken from main side (per design conflict resolution policy), which omitted xtask's transitive dependencies. The first CI run (`24955674267`) failed with `cannot update the lock file ... because --locked was passed`. Fix commit: `a460c18d :wrench: Update Cargo.lock to reflect xtask dependencies after Stage 1 rebase`.

## Stage 2 newly-detected fail axes

Stage 2 (`6c4173bf`) added `uid`/`gid` to `FsEntry::File`/`Dir` and `mtime_secs` to `Dir`. The Stage 2 CI run (`24957086937`) returned all 6 jobs success; no new fail axes detected at the bsdtar_test integration level (Windows xtask build green, ubuntu/macos PNA parity green). Detailed xtask `bsdtar-compat` differential outcomes were not recorded at this stage; see Stage 3 below for the first full xtask run.

## Stage 3 newly-detected fail axes

Stage 3 (`a8ebb8ef`) added Dereference axis (`-L`) and 4 symlink-shape ArchiveEntryType variants (SymlinkToDir, SymlinkChainShallow, SymlinkChainDeep, SymlinkDangling). The first full `cargo run -p xtask -- bsdtar-compat` execution on macOS (libarchive 3.5.3) produced:

```
317952 scenarios: 302921 passed, 14583 failed, 448 errors
```

### Fail breakdown by `<deref>_<entry-type>` (sorted by count)

| Deref + entry | Fail count | New variant? |
|---|---|---|
| `L_SymDir` | 1207 | yes |
| `no_L_SymChain4` | 1130 | yes |
| `no_L_SymChain2` | 1118 | yes |
| `L_Dir` | 1103 | no (existing) |
| `no_L_Dir` | 1094 | no |
| `L_Nested` | 992 | no |
| `no_L_Nested` | 992 | no |
| `no_L_SymDir` | 895 | yes |
| `no_L_Sym` | 794 | no |
| `L_SymDangling` | 694 | yes |
| `no_L_SymDangling` | 692 | yes |
| `L_SymChain4` | 649 | yes |
| `L_Sym` | 626 | no |
| `L_SymChain2` | 622 | yes |
| `L_HLink` | 597 | no |
| `no_L_HLink` | 583 | no |
| `no_L_File` | 405 | no |
| `L_File` | 390 | no |

### Errors

448 errors, dominated by pattern `no_L_File_over_Dir_keep_old_*: Permission denied (os error 13)`. This is an **existing PNA bug** unrelated to Dereference axis: when extract destination is a pre-existing Dir and `-k` (keep_old) is set, PNA attempts to write a File over the Dir without removing it first, hitting permission denied. Should be investigated as a separate issue.

### Investigation pointers

- The fact that `L_<X>` and `no_L_<X>` counts are similar for `Dir`, `Nested`, `Sym`, `HLink`, `SymDangling` indicates **Dereference 軸非依存の挙動差** for those entry types — the gap exists with or without `-L`.
- The new variants `SymDir`, `SymChain2`, `SymChain4`, `SymDangling` show meaningful gaps — these are the new coverage axes Stage 3 added. Each represents a class of bsdtar-divergence in PNA's create-time symlink handling.
- Detailed log: `/tmp/bsdtar-compat-L.log` (353692 lines on the run host).
- Per-axis investigation belongs to follow-up issues, not Stage 3.
