# Stage 4: xtask `-L` Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `xtask/src/bsdtar_compat.rs` with 4 new axes (FileSpec.mode, CmdlinePath, follow_command_links, timeout safeguard) so 6 `-L` test scenarios (L6, L10, L12, L15, L16, L17) become differential testing oracle scenarios. Permanently skip 3 scenarios (L13/L18/L19) per design.

**Architecture:** Four atomic axis additions, each in its own commit. (1) `FileSpec::File.mode` for permission fixture; (2) `CmdlinePath` enum for `-C src .` vs `-C src target`; (3) `follow_command_links` for `-H` orthogonal to `-L`; (4) `wait_timeout` integration in `run_cmd_capture` plus `SymlinkSelfLoop` / `SymlinkMutualLoop` `ArchiveEntryType` variants. Each addition follows the Stage 3 axis-extension pattern (enum/struct extension → generate_scenarios update → build_scenario_name update → consume site update).

**Tech Stack:** Rust 2024, xtask crate, `wait_timeout` crate (existing transitive dep via assert_cmd or new direct dep), `std::os::unix::fs::PermissionsExt`, libarchive bsdtar (oracle).

**Spec:** `docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md` (commit `234e88b4` on `ci/bsdtar-compat-labels`)

**Prerequisite:** Stages 1-3 complete (HEAD `234e88b4` includes Stage 4 design only, all framework changes from Stages 1-3 are landed).

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `xtask/src/bsdtar_compat.rs` | Modify | All axis/struct/scenario additions |
| `xtask/Cargo.toml` | Modify (only if `wait_timeout` not already pulled in) | Add `wait-timeout = "0.2"` dependency |
| `docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md` | Modify | Append observed bsdtar loop behavior + L12 axis check + completion record |
| `docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md` | Modify | Append "Permanently deferred (out of xtask scope)" section listing L13/L18/L19 |

---

## Task 1: Pre-implementation observations

**Files:**
- Modify: `docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md` (append observation results)

- [ ] **Step 1: Observe bsdtar behavior on symlink self-loop**

```bash
WORK=$(mktemp -d) && cd "$WORK" && mkdir src && ln -s loop src/loop && timeout 5 bsdtar -cLf out.tar -C src . ; echo "exit=$?"
```

Record exit code, stderr, archive content (`bsdtar -tvf out.tar`). Expected outcomes:
- (a) Loop detected: exit non-zero, error stderr, no entry in archive
- (b) Loop hang: timeout 5 kills, exit 124
- (c) Symlink preserved (no actual loop attempted with `-L` for self-target): exit 0, symlink in archive

- [ ] **Step 2: Observe bsdtar behavior on mutual loop**

```bash
WORK=$(mktemp -d) && cd "$WORK" && mkdir src && ln -s b src/a && ln -s a src/b && timeout 5 bsdtar -cLf out.tar -C src . ; echo "exit=$?"
```

Same recording as Step 1.

- [ ] **Step 3: Confirm `wait_timeout` availability**

Run:
```bash
grep -E '^name = "wait[-_]timeout"' Cargo.lock | head -3
```

Expected: a line `name = "wait-timeout"` (already transitive via assert_cmd 2.x). If absent, Step 4 adds it as direct xtask dep.

- [ ] **Step 4: Add wait_timeout to xtask deps if missing**

If Step 3 found nothing, edit `xtask/Cargo.toml`:

```toml
[dependencies]
# ... existing ...
wait-timeout = "0.2"
```

Otherwise skip this step.

- [ ] **Step 5: Confirm L12 axis coverage by existing `mtime_relation`**

Read `xtask/src/bsdtar_compat.rs` lines 177-183 (`MtimeRelation` enum) and grep for its uses:

```bash
grep -n "MtimeRelation::ArchiveNewer\|MtimeRelation::ArchiveOlder\|MtimeRelation::Irrelevant" xtask/src/bsdtar_compat.rs | head -10
```

Expected: existing axis is already used in `make_source_files`/`make_pre_existing` to control fixture mtime against `KeepNewerFiles` extract logic. **Conclusion: L12 (target_mtime_archived) is already covered by `mtime_relation` axis combined with the no_L_/L_ Sym variants — no additional work needed for L12.**

- [ ] **Step 6: Append observations to design spec**

Edit `docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md`. Append a new section before `## Related specs`:

```markdown
## Pre-implementation observation results (recorded YYYY-MM-DD)

### bsdtar self-loop (`-cLf` with `src/loop -> loop`)
- Exit code: <FROM_STEP_1>
- Stderr: <FROM_STEP_1>
- Archive entries: <FROM_STEP_1>

### bsdtar mutual loop (`-cLf` with `a -> b, b -> a`)
- Exit code: <FROM_STEP_2>
- Stderr: <FROM_STEP_2>
- Archive entries: <FROM_STEP_2>

### Decision on timeout safeguard
Based on observation: <if hang observed → timeout 30s required; if immediate error → timeout still recommended for defensive purposes; if symlink preserved → timeout not strictly necessary but kept for future-proofing>.

### L12 axis check
`MtimeRelation::{Irrelevant, ArchiveNewer, ArchiveOlder}` already controls fixture mtime in conjunction with `Sym`/`SymDir`/`SymChain*` entry types. L12 (target_mtime_archived) is implicit in the existing `*_arc_newer` / `*_arc_older` scenario suffixes. No new entry type or axis required for L12.
```

Replace `<FROM_STEP_1>` and `<FROM_STEP_2>` with the observed values verbatim.

- [ ] **Step 7: Commit pre-implementation findings**

```bash
git add docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md
[ -f xtask/Cargo.toml ] && git diff --quiet xtask/Cargo.toml || git add xtask/Cargo.toml
git commit -m ":memo: Inline bsdtar loop observations and L12 axis check into Stage 4 spec"
```

---

## Task 2: Add `FileSpec::File.mode` for permission fixture (L10)

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs` lines 305-380 (`FileSpec` enum) and lines 383-440 (`materialize`)

- [ ] **Step 1: Add `mode` field to `FileSpec::File`**

Locate `enum FileSpec` (around line 305) and replace the `File` variant with:

```rust
    File {
        path: &'static str,
        contents: &'static [u8],
        mtime_epoch: Option<i64>,
        mode: Option<u32>,
    },
```

- [ ] **Step 2: Update `materialize` to apply `mode`**

In `fn materialize` (around line 383), replace the `File` arm with:

```rust
            FileSpec::File {
                path,
                contents,
                mtime_epoch,
                mode,
            } => {
                let full = root.join(path);
                if let Some(parent) = full.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full, contents)?;
                if let Some(m) = mode {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&full, fs::Permissions::from_mode(*m))?;
                }
                let epoch = mtime_epoch.unwrap_or(DEFAULT_MTIME);
                let time = epoch_to_system_time(epoch);
                let file = fs::File::options().write(true).open(&full)?;
                file.set_modified(time)?;
            }
```

- [ ] **Step 3: Update all `FileSpec::File { ... }` literal callsites to add `mode: None`**

Run:
```bash
grep -n 'FileSpec::File {' xtask/src/bsdtar_compat.rs | wc -l
```

Note count (probably 10-15). For each callsite, add `mode: None,` after `mtime_epoch: ...,` line. Example diff:

```diff
            FileSpec::File {
                path: "target",
                contents: b"from_archive",
                mtime_epoch,
+               mode: None,
            },
```

Bulk approach with sed (verify result):

```bash
grep -B 0 -A 3 'FileSpec::File {' xtask/src/bsdtar_compat.rs | grep -c 'mode:' || echo "0"
```

Expected: returns `0` before this step. After completing all manual additions:

```bash
grep -B 0 -A 5 'FileSpec::File {' xtask/src/bsdtar_compat.rs | grep -c 'mode:'
```

Expected: equal to count from `grep -n 'FileSpec::File {' ...` above (each `File { ... }` literal now has its `mode:` line).

- [ ] **Step 4: Build to verify**

```bash
cargo build -p xtask 2>&1 | tail -5
```

Expected: `Finished ...`. If a `FileSpec::File` literal is still missing `mode`, build fails with `missing field 'mode'`. Add it and rebuild.

- [ ] **Step 5: Commit**

```bash
git add xtask/src/bsdtar_compat.rs
git commit -m ":sparkles: Add FileSpec::File.mode for permission fixture (Stage 4 L10)"
```

---

## Task 3: Add `CmdlinePath` axis (L6)

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs` (around lines 184-260 for axis enum, lines 196 for `CreateOptions`, generate_scenarios loop, build_scenario_name, run_scenario)

- [ ] **Step 1: Add `CmdlinePath` enum**

Locate the line just below `Dereference::label` impl (the empty line before the `// Sub-axis` comment block) and insert:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmdlinePath {
    TraverseInside,
    Explicit,
}

impl CmdlinePath {
    const ALL: &[Self] = &[Self::TraverseInside, Self::Explicit];

    fn label(self) -> &'static str {
        match self {
            Self::TraverseInside => "trav",
            Self::Explicit => "expl",
        }
    }
}
```

- [ ] **Step 2: Add `cmdline_path` field to `CreateOptions`**

Locate `struct CreateOptions` (around line 196) and replace with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CreateOptions {
    dereference: Dereference,
    cmdline_path: CmdlinePath,
}
```

- [ ] **Step 3: Update `generate_scenarios` to add the loop**

Locate `fn generate_scenarios` and find the `for &deref in Dereference::ALL` line. Wrap it in a new outer loop:

```diff
     for &deref in Dereference::ALL {
+        for &cmd_path in CmdlinePath::ALL {
         for &pre in PreExisting::ALL {
             ...
                                                         let create_options = CreateOptions {
                                                             dereference: deref,
+                                                            cmdline_path: cmd_path,
                                                         };
             ...
         }
+        }
     }
```

(Indentation will be off after the wrap; let `cargo fmt` fix it after the build verifies.)

- [ ] **Step 4: Update `build_scenario_name` to include `cmdline_path` label**

Locate `fn build_scenario_name` (around line 220) and replace the `format!` line:

```diff
     let mut name = format!(
-        "{}_{}_over_{}_{}",
+        "{}_{}_{}_over_{}_{}",
         create_opts.dereference.label(),
+        create_opts.cmdline_path.label(),
         entry.label(),
         pre.label(),
         opts.overwrite_mode.label()
     );
```

- [ ] **Step 5: Update `run_scenario` to use `cmdline_path` for `-cf` invocations**

In `fn run_scenario`, locate the bsdtar `-cf` `run_cmd` block (around line 818) and replace with:

```rust
    let create_path_arg = match scenario.create_options.cmdline_path {
        CmdlinePath::TraverseInside => ".",
        CmdlinePath::Explicit => "target",
    };

    run_cmd(
        Command::new(bsdtar_bin)
            .args(["-cf", bsdtar_archive.to_str().unwrap()])
            .args(&create_args)
            .arg("-C")
            .arg(&bsdtar_src)
            .arg(create_path_arg),
    )?;
```

Locate the pna `-cf` `run_cmd` block (around line 846) and replace similarly:

```rust
    run_cmd(
        Command::new(pna_bin)
            .args(["compat", "bsdtar", "--unstable"])
            .args(["-cf", pna_archive.to_str().unwrap()])
            .args(&create_args)
            .arg("-C")
            .arg(&pna_src)
            .arg(create_path_arg),
    )?;
```

- [ ] **Step 6: Build and apply `cargo fmt`**

```bash
cargo fmt -p xtask
cargo build -p xtask 2>&1 | tail -5
```

Expected: `Finished ...`. If build fails, examine error and adjust the relevant block.

- [ ] **Step 7: Commit**

```bash
git add xtask/src/bsdtar_compat.rs
git commit -m ":sparkles: Add CmdlinePath axis to xtask bsdtar-compat (Stage 4 L6)"
```

---

## Task 4: Add `follow_command_links` axis (L17)

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs` (`CreateOptions`, `make_create_args`, `generate_scenarios`, `build_scenario_name`)

- [ ] **Step 1: Add `follow_command_links` field to `CreateOptions`**

Replace `struct CreateOptions` definition again:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CreateOptions {
    dereference: Dereference,
    cmdline_path: CmdlinePath,
    follow_command_links: bool,
}
```

- [ ] **Step 2: Update `make_create_args` to emit `-H`**

Replace `fn make_create_args` body:

```rust
fn make_create_args(opts: &CreateOptions) -> Vec<&'static str> {
    let mut args = Vec::new();
    if opts.dereference == Dereference::WithDereference {
        args.push("-L");
    }
    if opts.follow_command_links {
        args.push("-H");
    }
    args
}
```

- [ ] **Step 3: Update `generate_scenarios` with new boolean loop**

In `fn generate_scenarios`, locate the existing `for &cmd_path in CmdlinePath::ALL` block (added in Task 3) and add an inner loop:

```diff
     for &cmd_path in CmdlinePath::ALL {
+        for follow_h in [false, true] {
         for &pre in PreExisting::ALL {
             ...
                                                         let create_options = CreateOptions {
                                                             dereference: deref,
                                                             cmdline_path: cmd_path,
+                                                            follow_command_links: follow_h,
                                                         };
             ...
         }
+        }
     }
```

- [ ] **Step 4: Update `build_scenario_name` to include `_H` suffix conditionally**

In `fn build_scenario_name`, after the existing `let mut name = format!(...)` line, add:

```rust
    if create_opts.follow_command_links {
        name.push_str("_H");
    }
```

Insert this immediately after the `let mut name = format!(...)` block (before `if opts.unlink_first { ... }`).

- [ ] **Step 5: Build**

```bash
cargo fmt -p xtask
cargo build -p xtask 2>&1 | tail -5
```

Expected: `Finished ...`.

- [ ] **Step 6: Commit**

```bash
git add xtask/src/bsdtar_compat.rs
git commit -m ":sparkles: Add follow_command_links axis to xtask bsdtar-compat (Stage 4 L17)"
```

---

## Task 5: Add loop entry types and timeout safeguard (L15/L16)

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs` (`ArchiveEntryType` enum, `make_source_files`, `ScenarioResult` enum, `run_cmd_capture` signature, `run_scenario` invocation sites)

- [ ] **Step 1: Add 2 new `ArchiveEntryType` variants**

Locate `enum ArchiveEntryType` and replace with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveEntryType {
    File,
    Directory,
    Symlink,
    SymlinkToDir,
    SymlinkChainShallow,
    SymlinkChainDeep,
    SymlinkDangling,
    SymlinkSelfLoop,
    SymlinkMutualLoop,
    HardLink,
    NestedPath,
}

impl ArchiveEntryType {
    const ALL: &[Self] = &[
        Self::File,
        Self::Directory,
        Self::Symlink,
        Self::SymlinkToDir,
        Self::SymlinkChainShallow,
        Self::SymlinkChainDeep,
        Self::SymlinkDangling,
        Self::SymlinkSelfLoop,
        Self::SymlinkMutualLoop,
        Self::HardLink,
        Self::NestedPath,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Directory => "Dir",
            Self::Symlink => "Sym",
            Self::SymlinkToDir => "SymDir",
            Self::SymlinkChainShallow => "SymChain2",
            Self::SymlinkChainDeep => "SymChain4",
            Self::SymlinkDangling => "SymDangling",
            Self::SymlinkSelfLoop => "SymLoopSelf",
            Self::SymlinkMutualLoop => "SymLoopMutual",
            Self::HardLink => "HLink",
            Self::NestedPath => "Nested",
        }
    }
}
```

- [ ] **Step 2: Add fixtures for new entry types in `make_source_files`**

In `fn make_source_files`, after the existing `ArchiveEntryType::SymlinkDangling` arm, insert:

```rust
        ArchiveEntryType::SymlinkSelfLoop => vec![FileSpec::Symlink {
            path: "target",
            target: "target",
        }],
        ArchiveEntryType::SymlinkMutualLoop => vec![
            FileSpec::Symlink {
                path: "loop_a",
                target: "loop_b",
            },
            FileSpec::Symlink {
                path: "loop_b",
                target: "loop_a",
            },
            FileSpec::Symlink {
                path: "target",
                target: "loop_a",
            },
        ],
```

- [ ] **Step 3: Add `Timeout` variant to `ScenarioResult`**

Locate `enum ScenarioResult` (around line 760 or grep `enum ScenarioResult`) and add `Timeout` variant:

```rust
enum ScenarioResult {
    Pass,
    Fail(Vec<Diff>),
    ExitMismatch { bsdtar_ok: bool, pna_ok: bool },
    Timeout { side: &'static str },
}
```

- [ ] **Step 4: Add timeout parameter to `run_cmd_capture`**

Replace `fn run_cmd_capture` signature and body:

```rust
fn run_cmd_capture(
    cmd: &mut Command,
    timeout: Option<Duration>,
) -> io::Result<Option<CmdResult>> {
    use std::io::Read;
    use wait_timeout::ChildExt;

    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let status_opt = match timeout {
        Some(d) => child.wait_timeout(d).map_err(io::Error::other)?,
        None => Some(child.wait()?),
    };

    let Some(status) = status_opt else {
        let _ = child.kill();
        let _ = child.wait();
        return Ok(None);
    };

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut s) = child.stdout.take() {
        s.read_to_end(&mut stdout).ok();
    }
    if let Some(mut s) = child.stderr.take() {
        s.read_to_end(&mut stderr).ok();
    }

    Ok(Some(CmdResult {
        success: status.success(),
        stdout,
        stderr,
    }))
}
```

(The original signature returned `io::Result<CmdResult>` directly; now it returns `Option<CmdResult>` with `None` indicating timeout.)

- [ ] **Step 5: Update all `run_cmd_capture` call sites in `run_scenario`**

Determine timeout per scenario based on entry type:

```rust
    let scenario_timeout = match scenario.entry_type {
        ArchiveEntryType::SymlinkSelfLoop | ArchiveEntryType::SymlinkMutualLoop => {
            Some(Duration::from_secs(30))
        }
        _ => None,
    };
```

Add the above near `let extract_args = ...` in `run_scenario`.

Replace bsdtar extract `run_cmd_capture` site (was returning `CmdResult` directly) with:

```rust
    let bsdtar_result = run_cmd_capture(
        Command::new(bsdtar_bin)
            .args(["-xf", bsdtar_archive.to_str().unwrap()])
            .args(&extract_args)
            .arg("-C")
            .arg(&bsdtar_dst),
        scenario_timeout,
    )?;
    let Some(bsdtar_result) = bsdtar_result else {
        return Ok(ScenarioResult::Timeout { side: "bsdtar" });
    };
```

Replace pna extract `run_cmd_capture` site similarly:

```rust
    let pna_result = run_cmd_capture(
        Command::new(pna_bin)
            .args(["compat", "bsdtar", "--unstable"])
            .args(["-xf", pna_archive.to_str().unwrap()])
            .args(&extract_args)
            .arg("-C")
            .arg(&pna_dst),
        scenario_timeout,
    )?;
    let Some(pna_result) = pna_result else {
        return Ok(ScenarioResult::Timeout { side: "pna" });
    };
```

Note: `run_cmd` for `-cf` (create) is also potentially loop-prone for `SymlinkSelfLoop` / `SymlinkMutualLoop` with `-L`. Since `run_cmd` is currently `Result<()>` returning, decide policy: for Stage 4 scope, the loop is exercised at extract time after archive contains it (or fails to be created). If `-cf -L` itself loops, the test will hang on `run_cmd`. Convert the `-cf` calls to use `run_cmd_capture` with `scenario_timeout` instead and emit `ScenarioResult::Timeout { side: "bsdtar_create" }` / `"pna_create"`.

Replace bsdtar `-cf` `run_cmd` block:

```rust
    let bsdtar_create = run_cmd_capture(
        Command::new(bsdtar_bin)
            .args(["-cf", bsdtar_archive.to_str().unwrap()])
            .args(&create_args)
            .arg("-C")
            .arg(&bsdtar_src)
            .arg(create_path_arg),
        scenario_timeout,
    )?;
    let Some(bsdtar_create) = bsdtar_create else {
        return Ok(ScenarioResult::Timeout { side: "bsdtar_create" });
    };
    if !bsdtar_create.success {
        return Err(format!(
            "bsdtar create failed: stderr={}",
            String::from_utf8_lossy(&bsdtar_create.stderr)
        )
        .into());
    }
```

Replace pna `-cf` `run_cmd` block similarly with `side: "pna_create"`.

- [ ] **Step 6: Update output formatter to handle `Timeout`**

Find where `ScenarioResult` is rendered in CLI output (grep `match res` or `ScenarioResult::Pass`). Add the `Timeout` case:

```rust
        ScenarioResult::Timeout { side } => {
            println!("[TIMEOUT:{side}] {}", scenario.name);
        }
```

- [ ] **Step 7: Build**

```bash
cargo fmt -p xtask
cargo build -p xtask 2>&1 | tail -10
```

Expected: `Finished ...`. Common errors:
- `wait_timeout` not in scope: add `use wait_timeout::ChildExt;` at function level (Step 4 already includes it).
- `wait_timeout` not in deps: revisit Task 1 Step 4.

- [ ] **Step 8: Commit**

```bash
git add xtask/src/bsdtar_compat.rs
git commit -m ":sparkles: Add SymlinkSelfLoop/MutualLoop entry types with timeout safeguard (Stage 4 L15/L16)"
```

---

## Task 6: Run xtask oracle and record outcome

**Files:**
- Modify: `docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md` (append outcome)
- Modify: `docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md` (append Stage 4 section)

- [ ] **Step 1: Run xtask in release mode**

```bash
cargo run -p xtask --release -- bsdtar-compat 2>&1 | tee /tmp/bsdtar-compat-stage4.log | tail -5
```

Expected: long-running. Final line should match pattern `<N> scenarios: <P> passed, <F> failed, <E> errors` (and possibly `<T> timeouts` if your output formatter was extended).

- [ ] **Step 2: Aggregate fail/timeout breakdown**

Run:
```bash
grep -c '^\[PASS\]' /tmp/bsdtar-compat-stage4.log
grep -c '^\[FAIL\]' /tmp/bsdtar-compat-stage4.log
grep -c '^\[ERROR\]' /tmp/bsdtar-compat-stage4.log
grep -c '^\[TIMEOUT' /tmp/bsdtar-compat-stage4.log
grep '^\[FAIL\]' /tmp/bsdtar-compat-stage4.log | awk '{print $2}' | sed -E 's/^(no_L|L)_(trav|expl)_([A-Za-z0-9]+)_.*/\1_\2_\3/' | sort | uniq -c | sort -rn | head -25 > /tmp/stage4-fail-breakdown.txt
grep '^\[TIMEOUT' /tmp/bsdtar-compat-stage4.log | head -10 > /tmp/stage4-timeout-sample.txt
```

- [ ] **Step 3: Append outcome to design spec**

Edit `docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md`. Append a new section after `## Pre-implementation observation results`:

```markdown
## Implementation Outcome (Stage 4)

- **Date**: <YYYY-MM-DD>
- **HEAD at run**: <git rev-parse HEAD>
- **Run summary**: <FROM_STEP_1_LAST_LINE>
- **Run log**: `/tmp/bsdtar-compat-stage4.log`

### Pass / Fail / Error / Timeout counts

| Result | Count |
|---|---|
| PASS | <FROM_STEP_2> |
| FAIL | <FROM_STEP_2> |
| ERROR | <FROM_STEP_2> |
| TIMEOUT | <FROM_STEP_2> |

### Fail breakdown by `<deref>_<cmdline_path>_<entry-type>` (top 25)

```
<CONTENTS_OF_/tmp/stage4-fail-breakdown.txt>
```

### Timeout samples (top 10)

```
<CONTENTS_OF_/tmp/stage4-timeout-sample.txt>
```

### New L-axis scenarios introduced by Stage 4

| Spec scenario | Pattern | Status |
|---|---|---|
| L6 (`L_symlink_explicit_in_cmdline`) | `*_expl_*` (cmdline_path=Explicit) | covered by axis |
| L10 (`L_target_permission_archived`) | `*_File_*_mode<N>` (would require fixture mode set; current scenarios cover File without mode, mode-bearing fixtures must be added separately) | partial — see follow-up note |
| L12 (`L_target_mtime_archived`) | `*_arc_newer` / `*_arc_older` × `Sym*` | covered by existing mtime_relation axis |
| L15 (`L_symlink_loop_self`) | `*_SymLoopSelf_*` | covered by new entry type |
| L16 (`L_symlink_loop_mutual`) | `*_SymLoopMutual_*` | covered by new entry type |
| L17 (`L_and_H_both_specified`) | `*_*_H` (follow_command_links=true) | covered by axis |
```

Replace `<...>` placeholders with concrete values from Step 2's output files.

- [ ] **Step 4: Append Stage 4 section to fail axis tracker**

Edit `docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md`. Append after the existing Stage 3 section:

```markdown
## Stage 4 newly-detected fail / timeout axes

Stage 4 (`<HEAD>`) added 4 new axes (`FileSpec.mode`, `CmdlinePath`, `follow_command_links`, timeout safeguard) and 2 new entry types (`SymlinkSelfLoop`, `SymlinkMutualLoop`). The first xtask oracle run produced:

```
<FROM_STEP_1_LAST_LINE>
```

### Fail breakdown (top 25 by axis combination)

```
<CONTENTS_OF_/tmp/stage4-fail-breakdown.txt>
```

### Timeout samples

```
<CONTENTS_OF_/tmp/stage4-timeout-sample.txt>
```

## Permanently deferred (out of xtask scope)

The following `-L` test scenarios are out of scope for the xtask `bsdtar-compat` framework. They will not be added in any future stage of this design lineage:

- **L13** (`L_target_uid_gid_archived`): `chown(2)` requires root or fakeroot. xtask runs as the regular CI user, and adding fakeroot to the test workflow conflicts with xtask's standalone-binary execution model.
- **L18** (`L_windows_reparse_point`): Stage 2 sealed `xtask::bsdtar_compat` under `#[cfg(unix)]`. Windows reparse-point fixture creation requires Windows API and contradicts the cfg-sealing decision.
- **L19** (`L_broken_symlink_warning_format`): warning text is locale (`LANG`/`LC_*`) and libarchive-version dependent. Adding a `StderrSnapshot` mechanism would be high-cost and produce flaky comparisons.

These scenarios should not be re-opened without first revising the design constraints listed above.
```

Replace placeholders with concrete values.

- [ ] **Step 5: Commit outcome and tracker updates**

```bash
git add docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md
git commit -m ":memo: Record Stage 4 xtask -L extension run outcome and permanent skip list"
```

---

## Task 7: Push and verify CI

**Files:** None directly. May append a final completion note to design spec.

- [ ] **Step 1: Push commits**

```bash
git push origin ci/bsdtar-compat-labels
```

Expected: fast-forward push, output includes the new commit range.

- [ ] **Step 2: Wait for CI to register and watch the bsdtar-compat run**

```bash
sleep 12
gh run list --branch ci/bsdtar-compat-labels --workflow "bsdtar compatibility" --limit 3 --json databaseId,headSha,status
```

Identify the run ID for the new HEAD. Run:

```bash
gh run watch <RUN_ID> --exit-status 2>&1 | tail -10 || true
```

- [ ] **Step 3: Verify CI conclusion**

```bash
gh run view <RUN_ID> --json status,conclusion,jobs 2>&1 | jq -c '{c: .conclusion, jobs: [.jobs[] | {n: .name, c: .conclusion}]}'
```

Expected: all 6 jobs `success` (3 platform × verify + test). If any job fails, halt and inspect:

```bash
gh run view <RUN_ID> --log-failed | head -60
```

- [ ] **Step 4: Append completion record to design spec**

Edit `docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md`. Append:

```markdown
## Stage 4 Completion Record

- **Date**: <YYYY-MM-DD>
- **Final HEAD**: <git rev-parse HEAD>
- **CI run**: <RUN_ID> `bsdtar compatibility`
- **Conclusion**: success (all 6 jobs)
- **xtask run summary**: <FROM_TASK_6_STEP_1>
- **L scope coverage**: L1-L9, L11-L12, L14-L17 covered by xtask oracle; L13/L18/L19 permanently deferred (see tracker md)
```

- [ ] **Step 5: Commit completion record and push**

```bash
git add docs/plans/2026-04-26-Stage4-xtask-l-extension-design.md
git commit -m ":memo: Record Stage 4 CI outcome and completion status"
git push origin ci/bsdtar-compat-labels
```

---

## Self-Review

| Check | Result |
|---|---|
| **Spec coverage** | L10 → Task 2; L6 → Task 3; L17 → Task 4; L15/L16 → Task 5; L12 covered by existing axis (Task 1 Step 5 verified); L13/L18/L19 documented as permanently deferred (Task 6 Step 4). All 9 spec scenarios accounted for. ✅ |
| **Placeholder scan** | `<RUN_ID>`, `<FROM_STEP_*>`, `<YYYY-MM-DD>`, `<HEAD>`, `<CONTENTS_OF_*>` are explicit shell-substitution / runtime-fill placeholders. No `TBD`/`TODO` undefined steps. ✅ |
| **Type / method consistency** | `CmdlinePath`, `follow_command_links`, `mode`, `Timeout`, `SymlinkSelfLoop`, `SymlinkMutualLoop` named consistently across tasks. `run_cmd_capture` signature change (`Option<CmdResult>`) propagates to all call sites in Task 5 Step 5. `wait_timeout::ChildExt` import is at function scope to avoid polluting other modules. ✅ |
| **Frequent commits** | Task 1 (pre-impl), Task 2 (mode), Task 3 (CmdlinePath), Task 4 (-H), Task 5 (loop+timeout), Task 6 (outcome), Task 7 (CI completion) — 7 logical commits, each independently bisectable. ✅ |
| **TDD note** | xtask is integration-runner code without unit-test framework. Verification at each task is `cargo build` followed by intermediate xtask invocation in Task 6 (full run). This matches the Stage 3 pattern and is acceptable. ✅ |
