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

(To be appended by Stage 2 Task 4 if new fail axes emerge post-Stage-2 deployment. Currently empty.)
