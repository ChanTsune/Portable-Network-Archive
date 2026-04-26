# Stage 2: FsSnapshot Metadata Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `FsEntry::File` with `uid`/`gid`, extend `FsEntry::Dir` with `mtime_secs`/`uid`/`gid`, update `FsSnapshot::walk` to capture them, update `Display` impl, and seal `xtask/src/bsdtar_compat.rs` under `#[cfg(unix)]` so Windows runner can build xtask without the bsdtar-compat subcommand.

**Architecture:** Two atomic commits. Commit 1 adds `#[cfg(unix)]` to `mod bsdtar_compat;` and the `Command::BsdtarCompat` variant + match arm in `xtask/src/main.rs`, isolating Windows build sealing as bisectable change. Commit 2 extends `FsEntry` variants and the walk/Display logic, isolating the metadata capture logic. The two changes touch disjoint code regions and can be reasoned about independently.

**Tech Stack:** Rust 2024, `std::os::unix::fs::MetadataExt` (Unix only), cargo workspace with xtask crate.

**Spec:** `docs/plans/2026-04-26-Stage2-fs-snapshot-extension-design.md` (commit `607c72fc` on `ci/bsdtar-compat-labels`)

**Prerequisite:** Stage 1 (`ci/bsdtar-compat-labels` rebased onto `main`) must be complete. The current HEAD `607c72fc` is post-Stage-1.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `xtask/src/main.rs` | Modify (3 `#[cfg(unix)]` insertions) | Seal `bsdtar_compat` module + Command variant + run() match arm |
| `xtask/src/bsdtar_compat.rs` | Modify | Extend `FsEntry::File` with uid/gid; extend `FsEntry::Dir` with mtime_secs/uid/gid; update `Display` impl; update `FsSnapshot::walk` to capture |
| `docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md` | Append (optional, only if new fail axes appear after Stage 2 push) | Add Stage 2 newly-detected fail axes to existing tracker |

---

## Task 1: Pre-flight checks

**Files:** None modified.

- [ ] **Step 1: Confirm working tree clean and on the right branch**

Run:
```bash
git status --short
git log --oneline -1
```

Expected: working tree clean except untracked `.claude` etc., HEAD is `607c72fc :memo: Add #[cfg(unix)] sealing scope to Stage 2 design` (or a later same-branch commit if Stage 1 Task 7 ran).

If branch is wrong: `git switch ci/bsdtar-compat-labels && git pull --ff-only`.

- [ ] **Step 2: Confirm xtask currently builds on the local Unix host**

Run:
```bash
cargo build -p xtask 2>&1 | tail -3
```

Expected: `Finished ...` (no error). This is the Stage 1 baseline; Task 2's `#[cfg(unix)]` change must not break it.

- [ ] **Step 3: Confirm `MetadataExt::uid` / `gid` / `mtime` exist on Unix**

Run:
```bash
rustdoc --version >/dev/null 2>&1 && echo "rustdoc ok"
grep -c 'fn uid\|fn gid\|fn mtime' "$HOME/.rustup/toolchains/$(rustup default | cut -d' ' -f1)/share/doc/rust/html/std/os/unix/fs/trait.MetadataExt.html" 2>/dev/null || echo "rustdoc offline; trust std API contract"
```

Expected: either a count > 0 or the offline message. The `MetadataExt` API surface is stable in std since 1.0; this is a sanity check, not a blocker.

---

## Task 2: Seal `bsdtar_compat` module under `#[cfg(unix)]`

**Files:**
- Modify: `xtask/src/main.rs` (line 1, 47, 25)

- [ ] **Step 1: Read the current main.rs head to confirm line numbers**

Run:
```bash
sed -n '1,30p' xtask/src/main.rs
```

Confirm `mod bsdtar_compat;` is at line 1 and `Command::BsdtarCompat(args) => bsdtar_compat::run(args),` is around line 25 (current verified location). Note actual line numbers if they shifted.

- [ ] **Step 2: Add `#[cfg(unix)]` to module declaration**

Edit `xtask/src/main.rs` line 1, replacing:
```rust
mod bsdtar_compat;
```
with:
```rust
#[cfg(unix)]
mod bsdtar_compat;
```

- [ ] **Step 3: Add `#[cfg(unix)]` to the `BsdtarCompat` enum variant**

Edit `xtask/src/main.rs` around line 47 inside `enum Command { ... }`, replacing:
```rust
    /// Verify extraction behavior matches bsdtar
    BsdtarCompat(bsdtar_compat::BsdtarCompatArgs),
```
with:
```rust
    /// Verify extraction behavior matches bsdtar
    #[cfg(unix)]
    BsdtarCompat(bsdtar_compat::BsdtarCompatArgs),
```

- [ ] **Step 4: Add `#[cfg(unix)]` to the `run()` match arm**

Edit `xtask/src/main.rs` around line 25 inside the `match args.command { ... }`, replacing:
```rust
        Command::BsdtarCompat(args) => bsdtar_compat::run(args),
```
with:
```rust
        #[cfg(unix)]
        Command::BsdtarCompat(args) => bsdtar_compat::run(args),
```

- [ ] **Step 5: Verify Unix build is unbroken**

Run:
```bash
cargo build -p xtask 2>&1 | tail -3
```

Expected: `Finished ...`. The cfg gates are no-ops on Unix (target_family=unix), so behavior unchanged.

- [ ] **Step 6: Verify Windows build would not require the bsdtar_compat module**

Run:
```bash
cargo check -p xtask --target x86_64-pc-windows-gnu 2>&1 | tail -10 || echo "windows-gnu target not installed, skip"
```

Expected: either `Finished ...` (if windows-gnu target is installed) or "skip". This step is a local convenience check; the authoritative Windows build verification is in Task 4 via CI on PR #3002.

- [ ] **Step 7: Commit the cfg sealing**

Run:
```bash
git add xtask/src/main.rs
git commit -m ":recycle: Seal xtask bsdtar-compat module under #[cfg(unix)]"
```

Expected: a single commit on top of `607c72fc`. Verify with:
```bash
git log origin/main..HEAD --oneline | head -5
```

---

## Task 3: Extend `FsEntry` and update walk/Display

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs` lines 554-587 (`FsEntry` enum + `Display` impl), 599-630 (`FsSnapshot::walk`)

- [ ] **Step 1: Locate the current `FsEntry` enum and confirm structure**

Run:
```bash
sed -n '554,590p' xtask/src/bsdtar_compat.rs
```

Expected output (roughly):
```rust
enum FsEntry {
    File {
        contents: Vec<u8>,
        mode: u32,
        mtime_secs: i64,
    },
    Dir {
        mode: u32,
    },
    Symlink {
        target: PathBuf,
    },
}

impl std::fmt::Display for FsEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsEntry::File { contents, mode, mtime_secs } => match std::str::from_utf8(contents) {
                Ok(s) => write!(f, "File({s:?}, mode={mode:04o}, mtime={mtime_secs})"),
                Err(_) => write!(f, "File({} bytes, mode={mode:04o}, mtime={mtime_secs})", contents.len()),
            },
            FsEntry::Dir { mode } => write!(f, "Dir(mode={mode:04o})"),
            FsEntry::Symlink { target } => write!(f, "Symlink({})", target.display()),
        }
    }
}
```

If the structure differs significantly from the above, halt and consult the user — the spec assumes this baseline.

- [ ] **Step 2: Replace `FsEntry` enum with extended variants**

Edit `xtask/src/bsdtar_compat.rs` at line 554-566, replacing the `FsEntry` enum with:

```rust
enum FsEntry {
    File {
        contents: Vec<u8>,
        mode: u32,
        mtime_secs: i64,
        uid: u32,
        gid: u32,
    },
    Dir {
        mode: u32,
        mtime_secs: i64,
        uid: u32,
        gid: u32,
    },
    Symlink {
        target: PathBuf,
    },
}
```

- [ ] **Step 3: Update `Display` impl to render new fields**

Edit `xtask/src/bsdtar_compat.rs` at line 568-587 (the `impl std::fmt::Display for FsEntry` block), replacing it with:

```rust
impl std::fmt::Display for FsEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsEntry::File {
                contents,
                mode,
                mtime_secs,
                uid,
                gid,
            } => match std::str::from_utf8(contents) {
                Ok(s) => write!(
                    f,
                    "File({s:?}, mode={mode:04o}, mtime={mtime_secs}, uid={uid}, gid={gid})"
                ),
                Err(_) => write!(
                    f,
                    "File({} bytes, mode={mode:04o}, mtime={mtime_secs}, uid={uid}, gid={gid})",
                    contents.len()
                ),
            },
            FsEntry::Dir {
                mode,
                mtime_secs,
                uid,
                gid,
            } => write!(
                f,
                "Dir(mode={mode:04o}, mtime={mtime_secs}, uid={uid}, gid={gid})"
            ),
            FsEntry::Symlink { target } => write!(f, "Symlink({})", target.display()),
        }
    }
}
```

- [ ] **Step 4: Update `FsSnapshot::walk` to capture new fields**

Locate `fn walk` at line 599 and replace its body (lines 600-629) with:

```rust
    fn walk(root: &Path, dir: &Path, entries: &mut BTreeMap<PathBuf, FsEntry>) -> io::Result<()> {
        let mut dir_entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
        dir_entries.sort_by_key(|e| e.file_name());

        for entry in dir_entries {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            let meta = fs::symlink_metadata(&path)?;

            if meta.is_symlink() {
                let target = fs::read_link(&path)?;
                entries.insert(rel, FsEntry::Symlink { target });
            } else if meta.is_dir() {
                let mode = meta.mode() & 0o7777;
                let mtime_secs = meta.mtime();
                let uid = meta.uid();
                let gid = meta.gid();
                entries.insert(
                    rel.clone(),
                    FsEntry::Dir {
                        mode,
                        mtime_secs,
                        uid,
                        gid,
                    },
                );
                Self::walk(root, &path, entries)?;
            } else {
                let contents = fs::read(&path)?;
                let mode = meta.mode() & 0o7777;
                let mtime_secs = meta.mtime();
                let uid = meta.uid();
                let gid = meta.gid();
                entries.insert(
                    rel,
                    FsEntry::File {
                        contents,
                        mode,
                        mtime_secs,
                        uid,
                        gid,
                    },
                );
            }
        }
        Ok(())
    }
```

- [ ] **Step 5: Build to verify all changes compile**

Run:
```bash
cargo build -p xtask 2>&1 | tail -10
```

Expected: `Finished ...`. If build fails:
- Pattern-match error on missing field, fix by adding the field at the destructuring site.
- If error is unrelated (e.g., main-side change), halt and consult user.

- [ ] **Step 6: Verify smoke test still works**

Run:
```bash
cargo run -p xtask -- bsdtar-compat --help 2>&1 | head -10
```

Expected: same usage output as before (`Verify extraction behavior matches bsdtar`, `Usage: xtask bsdtar-compat ...`). The CLI surface is unchanged.

- [ ] **Step 7: Commit the metadata extension**

Run:
```bash
git add xtask/src/bsdtar_compat.rs
git commit -m ":sparkles: Capture uid, gid, and dir mtime in FsSnapshot for bsdtar-compat oracle"
```

Expected: a second commit on top of Task 2's `#[cfg(unix)]` sealing commit. Verify:
```bash
git log origin/main..HEAD --oneline | head -5
```
Expected: 5 lines (the rebase squash + alias replacement + design docs + Task 2 sealing + Task 3 extension).

---

## Task 4: Push and CI verification

**Files:** None modified directly. May append to `docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md` if it exists.

- [ ] **Step 1: Confirm no open PR conflicts**

Run:
```bash
gh pr list --head ci/bsdtar-compat-labels --json number,state,title
```

Expected: `[{"number":3002, "state":"OPEN", ...}]`. PR #3002 will receive new commits; not a conflict.

- [ ] **Step 2: Push to origin**

Run:
```bash
git push origin ci/bsdtar-compat-labels
```

Expected: a fast-forward push (no force needed since Task 2 and Task 3 commits append to existing HEAD). Output ends with `<old>...<new>  ci/bsdtar-compat-labels -> ci/bsdtar-compat-labels`.

- [ ] **Step 3: Wait for CI workflows to start**

Run:
```bash
sleep 10
gh run list --branch ci/bsdtar-compat-labels --limit 5 --json databaseId,name,status,event,headSha
```

Expected: at least one new `pull_request` event run for the new HEAD. Note any `bsdtar compatibility` workflow run ID for Step 4.

If `bsdtar compatibility` does not appear (it has restricted trigger paths in `.github/workflows/bsdtar-compat.yml`), the user may need to dispatch manually with admin permissions:
```
gh workflow run "bsdtar compatibility" --ref ci/bsdtar-compat-labels
```
Halt and ask for that dispatch if needed.

- [ ] **Step 4: Watch the bsdtar-compat run**

Identify the bsdtar-compat run ID as `$RUN_ID` from Step 3. Run:
```bash
gh run watch "$RUN_ID" --exit-status 2>&1 | tail -20 || true
```

The `|| true` is intentional (Stage 1 option C: fail axes are tracked, not blocked).

- [ ] **Step 5: Compare new fail axes against Stage 1 baseline**

Run:
```bash
gh run view "$RUN_ID" --log-failed 2>/dev/null | grep -E '^\[FAIL\]' | sort -u > /tmp/stage2-fail-axis.txt
wc -l /tmp/stage2-fail-axis.txt
```

If a Stage 1 baseline `tracker md` exists at `docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md`, run:
```bash
ls docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md && \
  diff <(grep '`L_\\|`unlink_\\|`File_\\|`Dir_\\|`Sym_' docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md | sed 's/.*`\([^`]*\)`.*/\1/' | sort -u) /tmp/stage2-fail-axis.txt | head -30
```

This shows axes added or removed by Stage 2.

- [ ] **Step 6: Append Stage 2 newly-detected fail axes to tracker (if tracker exists)**

If `docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md` exists and Step 5 found new fail axes, append a Stage 2 section:

```bash
if [ -f docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md ]; then
  cat >> docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md <<EOF

## Stage 2 Newly-Detected Fail Axes

Stage 2 added uid/gid capture to FsSnapshot. The following axes newly fail (or fail differently) after Stage 2 push:

| # | Scenario | Notes |
|---|---|---|
EOF
  awk 'BEGIN{i=1} /^\[FAIL\]/ {print "| " i " | \`" $2 "\` | new in Stage 2; metadata diff suspected |"; i++}' /tmp/stage2-fail-axis.txt >> docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md
fi
```

If the tracker file does not yet exist (Stage 1 Task 7 not yet completed), skip this step. The Stage 1 Task 7 work (creating the tracker) will absorb Stage 2 fail axes when run later.

- [ ] **Step 7: Commit tracker append (if applied) and push**

Run (only if Step 6 appended):
```bash
if git diff --quiet docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md 2>/dev/null; then
  echo "no tracker changes to commit"
else
  git add docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md
  git commit -m ":memo: Append Stage 2 fail axes to bsdtar-compat post-rebase tracker"
  git push origin ci/bsdtar-compat-labels
fi
```

---

## Self-Review

| Check | Result |
|---|---|
| **Spec coverage** | Goal "FsEntry::File に uid/gid 追加": Task 3 Step 2. Goal "FsEntry::Dir に mtime/uid/gid 追加": Task 3 Step 2. "FsSnapshot::walk で capture": Task 3 Step 4. "Display impl 更新": Task 3 Step 3. "#[cfg(unix)] sealing in main.rs": Task 2 Steps 2-4. "Build green": Task 2 Step 5, Task 3 Step 5. "Windows build green": Task 4 Steps 3-4 via PR #3002 CI. "Stage 3 prerequisite": implicit by enum field presence. "新規 fail axis 追跡": Task 4 Steps 5-7. ✅ |
| **Placeholder scan** | `$RUN_ID`, `<old>`, `<new>` are explicit shell substitutions. No `TBD`/`TODO`. ✅ |
| **Type/method consistency** | `uid`/`gid` consistently named (not `owner_uid` etc.). `mtime_secs` matches existing field name (not `mtime` or `modified`). `MetadataExt::uid()`/`gid()`/`mtime()` are the std-library method names used in Task 3 Step 4 and they match what `MetadataExt` provides on Unix. ✅ |
| **Frequent commits** | Task 2 commits sealing, Task 3 commits metadata extension, Task 4 Step 7 conditionally commits tracker append. Three logical commits, each independently bisectable. ✅ |
| **TDD note** | The xtask binary lacks a unit-test framework (no `#[cfg(test)]` modules in `bsdtar_compat.rs`). The integration verification is the bsdtar-compat run (Task 4 Step 4) — building the binary (Tasks 2/3 Step 5) is the proxy for "tests pass" before commit. Acceptable for this codebase. ✅ |
