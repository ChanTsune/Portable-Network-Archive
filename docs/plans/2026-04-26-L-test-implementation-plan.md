# `-L` / `--dereference` Verification Testing Implementation Plan (Stage 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `-L` / `--dereference` differential testing axis to `cargo xtask bsdtar-compat`, network-verifying parity with bsdtar via 13 oracle scenarios (Tier 1+2 minus L6) covering symlink dereference behavior.

**Architecture:** Extend existing `xtask/src/bsdtar_compat.rs` Scenario framework: (1) add 4 `ArchiveEntryType` variants for symlink shapes (SymlinkToDir, SymlinkChainShallow, SymlinkChainDeep, SymlinkDangling), (2) introduce `CreateOptions` struct with `Dereference` axis, (3) parametrize the `-cf` invocation in `run_scenario` with `make_create_args`, (4) extend `generate_scenarios` and `build_scenario_name`. L6 (cmdline-explicit path) and Tier 3 bats scenarios are out of scope for this plan.

**Tech Stack:** Rust 2024, cargo xtask, libarchive bsdtar (built per-PR via existing `bsdtar-compat.yml` workflow), tempfile

**Spec:** `docs/plans/2026-04-26-L-test-design.md` (commit `62c0e799` on `ci/bsdtar-compat-labels`)

**Prerequisite:** Stage 1 (`ci/bsdtar-compat-labels` rebased onto current `main`) and Stage 2 (FsSnapshot extension with `mode`/`mtime`/`uid`/`gid` fields) must be completed before L10/L12/L13 can pass. L1-L9, L11, L14 (11 scenarios) work without Stage 2. This plan flags each scenario's prerequisite explicitly.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `xtask/src/bsdtar_compat.rs` | Modify | All axis/struct/scenario additions |
| `docs/plans/2026-04-26-L-test-design.md` | Modify | Inline observed bsdtar dangling symlink behavior into Section 5 (L4) and Section 8 |

No new files. CI workflow (`.github/workflows/bsdtar-compat.yml`) builds libarchive per-PR via `actions/checkout@... libarchive/libarchive`, so no install step changes needed for bsdtar oracle — confirmed at lines 30-33.

---

## Task 1: Pre-implementation — bsdtar 実機 dangling symlink 挙動の実測

**Files:**
- Modify: `docs/plans/2026-04-26-L-test-design.md` (Section 5 L4 row, Section 8 dangling safeguards row)

**Why:** Spec Section 11 mandates pre-implementation observation. Without this, L4's expected behavior is unknown and the scenario cannot be encoded.

- [ ] **Step 1: Create observation script**

```bash
cat > /tmp/observe_bsdtar_L_dangling.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
WORK=$(mktemp -d)
cd "$WORK"
mkdir -p src
ln -s nonexistent src/dangling
echo "=== bsdtar -cLf with dangling symlink ==="
bsdtar -cLf out.tar -C src . 2>&1 || echo "exit=$?"
echo "=== archive content ==="
bsdtar -tvf out.tar 2>&1 || true
EOF
chmod +x /tmp/observe_bsdtar_L_dangling.sh
```

- [ ] **Step 2: Run on Linux (current host or CI runner)**

Run: `/tmp/observe_bsdtar_L_dangling.sh`

Expected output: one of three patterns —
1. Warning to stderr (`bsdtar: <path>: ...`) + exit 0 + entry NOT in archive
2. Warning + exit 1
3. Symlink preserved in archive (no dereference attempted)

Record exact stderr text, exit code, and `bsdtar -tvf` listing.

- [ ] **Step 3: Run on macOS (if accessible)**

Same script, same recording. If macOS bsdtar (which is libarchive too) differs from Linux bsdtar, document both.

- [ ] **Step 4: Inline observed behavior into spec**

Edit `docs/plans/2026-04-26-L-test-design.md` Section 5 row L4 and Section 8 "dangling symlink (L4)" row:

```diff
-| L4 | `L_dangling_symlink_with_L` | on | symlink → non-existent | **bsdtar 実機の挙動を pre-impl phase で実測**して oracle 化 (warn+skip / error / 保持 のいずれか) |
+| L4 | `L_dangling_symlink_with_L` | on | symlink → non-existent | bsdtar 実機実測結果: <PATTERN_FROM_STEP_2>. expected: <CONCRETE_EXPECTATION_DERIVED_FROM_OBSERVATION> |
```

- [ ] **Step 5: Commit**

```bash
git add docs/plans/2026-04-26-L-test-design.md
git commit -m ":memo: Inline observed bsdtar dangling symlink behavior into -L spec"
```

---

## Task 2: Add 4 new ArchiveEntryType variants

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs:50-76` (enum definition + ALL + label)

- [ ] **Step 1: Extend the enum**

Replace `xtask/src/bsdtar_compat.rs:49-76` with:

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
            Self::HardLink => "HLink",
            Self::NestedPath => "Nested",
        }
    }
}
```

- [ ] **Step 2: Build and verify compilation**

Run: `cargo build -p xtask 2>&1 | tail -5`

Expected: build fails with non-exhaustive match in `make_source_files`. This is intentional — Task 8 fixes it. Confirm the failure is exactly the match-arm error.

- [ ] **Step 3: Do NOT commit yet**

Compilation broken; commit after Task 8.

---

## Task 3: Add CreateOptions struct with Dereference axis

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs:160` (insert before `// Sub-axis` comment)

- [ ] **Step 1: Insert struct + axis enum**

Insert at `xtask/src/bsdtar_compat.rs:160` (before `MtimeRelation`):

```rust
// ---------------------------------------------------------------------------
// Axis: Create-time options (parallel to ExtractOptions)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dereference {
    WithoutDereference,
    WithDereference,
}

impl Dereference {
    const ALL: &[Self] = &[Self::WithoutDereference, Self::WithDereference];

    fn label(self) -> &'static str {
        match self {
            Self::WithoutDereference => "no_L",
            Self::WithDereference => "L",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CreateOptions {
    dereference: Dereference,
}

```

- [ ] **Step 2: Add `create_options` field to `GeneratedScenario`**

Replace `xtask/src/bsdtar_compat.rs:176-182`:

```rust
struct GeneratedScenario {
    name: String,
    pre_existing: PreExisting,
    entry_type: ArchiveEntryType,
    create_options: CreateOptions,
    options: ExtractOptions,
    mtime_relation: MtimeRelation,
}
```

- [ ] **Step 3: Build and verify compilation**

Run: `cargo build -p xtask 2>&1 | tail -10`

Expected: build still fails (Task 2 unfinished + new field reference errors in `generate_scenarios`/`run_scenario`). Confirm failures are limited to `create_options` initialization sites and the existing match.

- [ ] **Step 4: Do NOT commit yet**

---

## Task 4: Add `make_create_args` function

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs:507` (insert just before `make_extract_args`)

- [ ] **Step 1: Insert function**

Insert at `xtask/src/bsdtar_compat.rs:506` (immediately above `fn make_extract_args`):

```rust
fn make_create_args(opts: &CreateOptions) -> Vec<&'static str> {
    let mut args = Vec::new();
    if opts.dereference == Dereference::WithDereference {
        args.push("-L");
    }
    args
}

```

- [ ] **Step 2: Build**

Run: `cargo build -p xtask 2>&1 | tail -10`

Expected: still failing on Task 2/3 unrelated sites; this function compiles.

- [ ] **Step 3: Do NOT commit yet**

---

## Task 5: Plumb `make_create_args` into `run_scenario`

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs:735-770` (both bsdtar and pna `-cf` invocations)

- [ ] **Step 1: Modify bsdtar create invocation**

Replace `xtask/src/bsdtar_compat.rs:735-741`:

```rust
    let create_args = make_create_args(&scenario.create_options);

    run_cmd(
        Command::new(bsdtar_bin)
            .args(["-cf", bsdtar_archive.to_str().unwrap()])
            .args(&create_args)
            .arg("-C")
            .arg(&bsdtar_src)
            .arg("."),
    )?;
```

- [ ] **Step 2: Modify pna create invocation**

Replace `xtask/src/bsdtar_compat.rs:763-770` (the second `run_cmd` block creating `pna_archive`):

```rust
    run_cmd(
        Command::new(pna_bin)
            .args(["experimental", "stdio", "--unstable"])
            .args(["-cf", pna_archive.to_str().unwrap()])
            .args(&create_args)
            .arg("-C")
            .arg(&pna_src)
            .arg("."),
    )?;
```

- [ ] **Step 3: Build**

Run: `cargo build -p xtask 2>&1 | tail -10`

Expected: still failing on Task 2 (match arm) and Task 3 (`generate_scenarios` initialization). `run_scenario` itself compiles.

- [ ] **Step 4: Do NOT commit yet**

---

## Task 6: Extend `generate_scenarios` with CreateOptions loop

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs:234-299` (the nested-loop scenario generator)

- [ ] **Step 1: Add `Dereference::ALL` outer loop and propagate to GeneratedScenario**

Replace the body of `fn generate_scenarios` (lines 234-299):

```rust
fn generate_scenarios() -> Vec<GeneratedScenario> {
    let mut scenarios = Vec::new();

    for &deref in Dereference::ALL {
        for &pre in PreExisting::ALL {
            for &entry in ArchiveEntryType::ALL {
                for &ow_mode in OverwriteMode::ALL {
                    for unlink in [false, true] {
                        for abs_paths in [false, true] {
                            for safe_writes in [false, true] {
                                for no_mtime in [false, true] {
                                    for perm in [false, true] {
                                        for no_same_owner in [false, true] {
                                            for strip_components in STRIP_COMPONENTS_OPTIONS {
                                                for exclude in EXCLUDE_PATTERNS {
                                                    for substitution in SUBSTITUTIONS {
                                                        let create_options = CreateOptions { dereference: deref };
                                                        let options = ExtractOptions {
                                                            overwrite_mode: ow_mode,
                                                            unlink_first: unlink,
                                                            absolute_paths: abs_paths,
                                                            safe_writes,
                                                            no_preserve_mtime: no_mtime,
                                                            preserve_permissions: perm,
                                                            no_same_owner,
                                                            strip_components: *strip_components,
                                                            exclude: *exclude,
                                                            substitution: *substitution,
                                                        };

                                                        let mtime_variants = if ow_mode
                                                            == OverwriteMode::KeepNewerFiles
                                                            && pre != PreExisting::None
                                                        {
                                                            &[
                                                                MtimeRelation::ArchiveNewer,
                                                                MtimeRelation::ArchiveOlder,
                                                            ][..]
                                                        } else {
                                                            &[MtimeRelation::Irrelevant][..]
                                                        };

                                                        for &mtime_rel in mtime_variants {
                                                            let name = build_scenario_name(
                                                                pre, entry, &create_options, &options, mtime_rel,
                                                            );
                                                            scenarios.push(GeneratedScenario {
                                                                name,
                                                                pre_existing: pre,
                                                                entry_type: entry,
                                                                create_options,
                                                                options,
                                                                mtime_relation: mtime_rel,
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    scenarios
}
```

- [ ] **Step 2: Build**

Run: `cargo build -p xtask 2>&1 | tail -10`

Expected: failing on Task 2 match arm only + `build_scenario_name` signature (Task 7 fixes).

- [ ] **Step 3: Do NOT commit yet**

---

## Task 7: Update `build_scenario_name` to include CreateOptions label

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs:184-232`

- [ ] **Step 1: Add CreateOptions parameter and label prefix**

Replace `fn build_scenario_name` signature and body (lines 184-232):

```rust
fn build_scenario_name(
    pre: PreExisting,
    entry: ArchiveEntryType,
    create_opts: &CreateOptions,
    opts: &ExtractOptions,
    mtime: MtimeRelation,
) -> String {
    let mut name = format!(
        "{}_{}_over_{}_{}",
        create_opts.dereference.label(),
        entry.label(),
        pre.label(),
        opts.overwrite_mode.label()
    );
    if opts.unlink_first {
        name.push_str("_unlink-first");
    }
    if opts.absolute_paths {
        name.push_str("_absolute-paths");
    }
    if opts.safe_writes {
        name.push_str("_safe-writes");
    }
    if opts.no_preserve_mtime {
        name.push_str("_no-preserve-mtime");
    }
    if opts.preserve_permissions {
        name.push_str("_preserve-permissions");
    }
    if opts.no_same_owner {
        name.push_str("_no-same-owner");
    }
    if let Some(strip) = opts.strip_components {
        name.push('_');
        name.push_str(strip.label);
    }
    if let Some(excl) = opts.exclude {
        name.push('_');
        name.push_str(excl.label);
    }
    if let Some(subst) = opts.substitution {
        name.push('_');
        name.push_str(subst.label);
    }
    match mtime {
        MtimeRelation::Irrelevant => {}
        MtimeRelation::ArchiveNewer => name.push_str("_arc_newer"),
        MtimeRelation::ArchiveOlder => name.push_str("_arc_older"),
    }
    name
}
```

- [ ] **Step 2: Build**

Run: `cargo build -p xtask 2>&1 | tail -10`

Expected: still failing on Task 2 match-arm exhaustiveness only.

- [ ] **Step 3: Do NOT commit yet**

---

## Task 8: Extend `make_source_files` for new ArchiveEntryType variants

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs:393-444` (match arms in `make_source_files`)

- [ ] **Step 1: Add 4 new match arms**

Insert into the `match entry_type { ... }` block in `make_source_files` (between existing `Symlink` and `HardLink` arms):

```rust
        ArchiveEntryType::SymlinkToDir => vec![
            FileSpec::Dir {
                path: "symlink_dest_dir",
                mtime_epoch: None,
            },
            FileSpec::File {
                path: "symlink_dest_dir/inside.txt",
                contents: b"inside_target_dir",
                mtime_epoch: None,
            },
            FileSpec::Symlink {
                path: "target",
                target: "symlink_dest_dir",
            },
        ],
        ArchiveEntryType::SymlinkChainShallow => vec![
            FileSpec::File {
                path: "chain_final",
                contents: b"chain_final_content",
                mtime_epoch: None,
            },
            FileSpec::Symlink {
                path: "chain_b",
                target: "chain_final",
            },
            FileSpec::Symlink {
                path: "target",
                target: "chain_b",
            },
        ],
        ArchiveEntryType::SymlinkChainDeep => vec![
            FileSpec::File {
                path: "chain_final",
                contents: b"chain_deep_content",
                mtime_epoch: None,
            },
            FileSpec::Symlink {
                path: "chain_d",
                target: "chain_final",
            },
            FileSpec::Symlink {
                path: "chain_c",
                target: "chain_d",
            },
            FileSpec::Symlink {
                path: "chain_b",
                target: "chain_c",
            },
            FileSpec::Symlink {
                path: "target",
                target: "chain_b",
            },
        ],
        ArchiveEntryType::SymlinkDangling => vec![
            FileSpec::Symlink {
                path: "target",
                target: "nonexistent_target",
            },
        ],
```

- [ ] **Step 2: Build**

Run: `cargo build -p xtask 2>&1 | tail -10`

Expected: clean build, no warnings related to the new code.

- [ ] **Step 3: Commit framework changes**

```bash
git add xtask/src/bsdtar_compat.rs
git commit -m ":sparkles: Add Dereference axis and 4 symlink-shape ArchiveEntryType variants for bsdtar-compat"
```

---

## Task 9: Run `cargo xtask bsdtar-compat` and observe scenario outcomes

**Files:** None modified in this task — this is a verification step.

- [ ] **Step 1: Confirm bsdtar binary is available**

Run: `which bsdtar && bsdtar --version | head -1`

Expected: a path and a libarchive version line. If absent on macOS, `brew install libarchive && export PATH="$(brew --prefix libarchive)/bin:$PATH"`. On Linux, `sudo apt install libarchive-tools`.

- [ ] **Step 2: Run the full compat suite**

Run: `cargo run -p xtask -- bsdtar-compat 2>&1 | tee /tmp/bsdtar-compat-L.log`

Expected: a list of `[PASS] <name>` and `[FAIL] <name>` entries. The total should be ~2× the previous count (Dereference axis doubles).

- [ ] **Step 3: Filter for L-axis scenarios and review**

Run: `grep -E '^\[(PASS|FAIL)\] L_' /tmp/bsdtar-compat-L.log | wc -l` — should be > 0.

Run: `grep '^\[FAIL\] L_' /tmp/bsdtar-compat-L.log` to enumerate failures.

- [ ] **Step 4: Categorize failures**

For each `[FAIL] L_<name>`:

1. Identify the spec scenario (L1-L14) it corresponds to via the entry_type and option labels in the name
2. Check if it depends on Stage 2 (mode/mtime/uid/gid metadata): scenarios with `_preserve-permissions` (L10), or named with non-`Irrelevant` mtime (L12), or involving uid/gid (L13). These failures are **expected** until Stage 2 lands — record them in a `Stage2-blocked` bucket.
3. Other failures (L1-L9, L11, L14): these indicate a real PNA / bsdtar parity gap. Do NOT fix in this plan — record them in a `parity-gap` bucket for follow-up issues.
4. The `SymlinkDangling` × `WithDereference` scenario (L4) should match the observed bsdtar behavior recorded in Task 1. If PNA diverges, add to `parity-gap` bucket.

- [ ] **Step 5: Write outcome report**

Append a new section to `docs/plans/2026-04-26-L-test-design.md`:

```markdown
## Implementation Outcome (Stage 3)

Date: <YYYY-MM-DD>
xtask run log: see CI artifact / `/tmp/bsdtar-compat-L.log`

| Bucket | Count | Scenarios |
|---|---|---|
| Pass | N | ... |
| Stage2-blocked (expected fail until Stage 2) | M | L10, L12, L13 patterns |
| Parity-gap (real divergence, follow-up issue needed) | K | <list> |

Follow-up issues to file: <bullet list>
```

- [ ] **Step 6: Commit outcome report**

```bash
git add docs/plans/2026-04-26-L-test-design.md
git commit -m ":memo: Record Stage 3 -L axis xtask run outcome"
```

---

## Task 10: Document L6 (cmdline-explicit path) as bats-only

**Files:**
- Modify: `docs/plans/2026-04-26-L-test-design.md` Section 5 (Test Matrix Tier 1)

**Why:** xtask `run_scenario` always uses `-cf -C src .` (line 737-741, 763-770). L6's "command-line で symlink 直接指定" cannot be expressed without a new axis for explicit cmdline paths. Defer to Tier 3 bats supplement (Stage 4).

- [ ] **Step 1: Add note to L6 row in spec**

Edit Section 5 Tier 1 table row L6:

```diff
-| L6 | `L_symlink_explicit_in_cmdline` | on | command-line で symlink 直接指定 | 両者: archive 内 dereferenced |
+| L6 | `L_symlink_explicit_in_cmdline` | on | command-line で symlink 直接指定 | **Stage 3 では実装しない** (xtask は `-cf -C src .` 固定)。Stage 4 bats supplement に移行 |
```

- [ ] **Step 2: Add note to Section 4 Architecture**

Append to Stage 3 description in Section 4:

```diff
 Stage 3: -L axis 追加 (THIS SPEC が対象)
    ├ axis: WithoutDereference / WithDereference の dual run
    ├ 検証 scenario matrix (Tier 1/2/3 で 19 件)
    └ FsSnapshot 拡張 (Stage 2) に依存
+   ⚠️ L6 (cmdline-explicit path) は xtask framework の `-cf -C src .` 固定制約により Stage 3 oracle 範疇外。Stage 4 bats supplement で扱う
```

- [ ] **Step 3: Commit**

```bash
git add docs/plans/2026-04-26-L-test-design.md
git commit -m ":memo: Defer L6 to Stage 4 bats (xtask cmdline path constraint)"
```

---

## Task 11: Verify CI workflow runs the new scenarios

**Files:** None modified.

- [ ] **Step 1: Check workflow trigger paths**

Run: `cat .github/workflows/bsdtar-compat.yml | grep -A 5 'paths:' | head -20`

Expected: `xtask/src/**` is in trigger paths. The branch push to `ci/bsdtar-compat-labels` triggers the workflow.

- [ ] **Step 2: Push the branch and observe CI**

```bash
git push origin ci/bsdtar-compat-labels
```

Run: `gh run list --branch ci/bsdtar-compat-labels --limit 5`

Expected: a `bsdtar compatibility` run is queued or in progress for the new HEAD commit.

- [ ] **Step 3: Wait for CI completion**

Run: `gh run watch <run-id> --exit-status`

Expected exit: 0 if all PASS, non-zero if any FAIL. Stage2-blocked failures will cause the run to be non-zero — this is expected and documented in Task 9 Step 5's outcome report.

- [ ] **Step 4: If CI passes for non-Stage2-blocked scenarios, mark Stage 3 done**

Append to `docs/plans/2026-04-26-L-test-design.md` "Implementation Outcome" section:

```markdown
**CI status:** <run URL>, <pass/expected-fail summary>
**Stage 3 status:** done modulo Stage 2 prerequisites
```

Commit:

```bash
git add docs/plans/2026-04-26-L-test-design.md
git commit -m ":memo: Record Stage 3 CI outcome and completion status"
```

---

## Self-Review

| Check | Result |
|---|---|
| Spec coverage: each Tier 1+2 scenario maps to a task | L1-L14 covered by Task 9 outcome categorization. L6 explicitly deferred (Task 10). Tier 3 (L15-L19) is Stage 4 scope (out of plan). ✅ |
| Placeholder scan | No "TBD"/"TODO". `<YYYY-MM-DD>`/`<run URL>`/`<list>` are runtime-fill placeholders within explicit instruction-bearing blocks, not undefined steps. ✅ |
| Type/method consistency | `Dereference`, `CreateOptions`, `make_create_args`, `create_options` field name used consistently across Tasks 3-7. ✅ |
| TDD ordering | xtask oracle scenarios self-test on each run; framework changes (Tasks 2-8) don't fail in isolation but compile-error feedback at each step keeps the loop tight. Task 9 is the test execution stage. Acceptable for this codebase pattern. ✅ |
| Frequent commits | Tasks 1, 8, 9, 10, 11 commit. Tasks 2-7 deliberately defer commit because the framework refactor is mid-air across files; a single coherent commit at Task 8 is more reviewable than 6 broken-build intermediate commits. ✅ |
