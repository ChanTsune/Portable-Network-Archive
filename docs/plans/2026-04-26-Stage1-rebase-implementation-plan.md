# Stage 1 Rebase Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Squash all `ci/bsdtar-compat-labels` commits into 1 commit, rebase onto current `main`, replace deprecated `experimental stdio` alias with `compat bsdtar` in xtask, and force-push so the framework runs against current main.

**Architecture:** Three-step git surgery: (1) `git reset --soft` + single commit creation collapses 24 commits to 1; (2) `git rebase origin/main` reapplies that single commit on the latest main; (3) a separate `:recycle:` commit replaces 2 hardcoded subcommand alias references. Force-push with `--force-with-lease`. CI failures are documented in a tracker issue and explicitly out of scope.

**Tech Stack:** git, gh CLI, cargo, libarchive (oracle, available via `apt install libarchive-tools` on Linux)

**Spec:** `docs/plans/2026-04-26-Stage1-rebase-design.md` (commit `ce587cec` on `ci/bsdtar-compat-labels`)

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `xtask/src/bsdtar_compat.rs` | Modify (2 lines) | Replace `experimental stdio` with `compat bsdtar` at lines 765 and 774 (line numbers post-rebase may shift; identify by content) |
| (none other) | — | Squash and rebase do not touch source files; only git history is reshaped |

No design/plan docs are modified by this plan; they are already committed on `ci/bsdtar-compat-labels`.

---

## Task 1: Pre-flight checks

**Files:** None modified.

**Why:** Confirm safety preconditions before destructive git operations (`reset --soft`, `rebase`, `force-push`).

- [ ] **Step 1: Confirm no open PR depends on the current `ci/bsdtar-compat-labels` HEAD**

Run:
```bash
gh pr list --head ci/bsdtar-compat-labels --json number,title,state
```

Expected: `[]` (empty array). If non-empty, the PRs would have their commits invalidated by force-push — abort and consult the user.

- [ ] **Step 2: Confirm `bsdtar` binary is available locally for build sanity**

Run:
```bash
which bsdtar && bsdtar --version | head -1
```

Expected: a path and a `bsdtar 3.x.x - libarchive ...` line. If absent on Linux, `sudo apt install libarchive-tools`. On macOS, `brew install libarchive && export PATH="$(brew --prefix libarchive)/bin:$PATH"`.

This is only needed if Step in Task 5 will exercise it; build-only sanity (Task 5 Step 1) does not require bsdtar. Confirming early avoids surprise later.

- [ ] **Step 3: Confirm fetching origin main is up-to-date**

Run:
```bash
git fetch origin main
git log --oneline -1 origin/main
```

Expected: a recent commit hash and message. Note the hash for later steps.

- [ ] **Step 4: Confirm we are on `ci/bsdtar-compat-labels` with current HEAD**

Run:
```bash
git switch ci/bsdtar-compat-labels
git log --oneline -1
```

Expected: `ce587cec :memo: Add design doc for ci/bsdtar-compat-labels rebase to main (Stage 1)`. If different, sync first: `git pull --ff-only`.

- [ ] **Step 5: Confirm divergence count matches expected (24+ commits ahead)**

Run:
```bash
git log --oneline origin/main..HEAD | wc -l
```

Expected: 25 (24 framework/doc commits + 1 design commit added by this brainstorming session). If significantly different, abort and consult the user — the squash range may be wrong.

---

## Task 2: Squash all commits into 1

**Files:** Git history only.

- [ ] **Step 1: Determine merge-base with main**

Run:
```bash
MERGE_BASE=$(git merge-base HEAD origin/main)
echo "$MERGE_BASE"
```

Record the merge-base hash for the next step. Expected: a 40-char hex hash. The variable `$MERGE_BASE` should remain set for the subsequent commands in this task.

- [ ] **Step 2: Reset HEAD to merge-base, keeping all changes staged**

Run:
```bash
git reset --soft "$MERGE_BASE"
```

Expected: no output. Verify with `git log --oneline -1` showing the merge-base commit, and `git diff --cached --stat` showing all 25 commits' worth of changes staged.

- [ ] **Step 3: Verify staged changes match the original branch tip**

Run:
```bash
git diff --cached --stat | tail -1
git diff origin/ci/bsdtar-compat-labels --stat | tail -1
```

Expected: the second `diff` against `origin/ci/bsdtar-compat-labels` (which still points to `ce587cec`) should show **0 file changes / no output beyond a blank line**, confirming the staged state is identical to the pre-reset HEAD content.

- [ ] **Step 4: Create the single squashed commit**

Run:
```bash
git commit -m ":sparkles: Add bsdtar oracle compatibility testing framework"
```

Expected: a single commit is created. Verify:
```bash
git log origin/main..HEAD --oneline
```
Expected: 1 commit OR less, depending on how many commits in `origin/main..HEAD` (i.e., commits on the branch but not on main). Since we squashed everything that was on the branch but not at the merge-base, after Step 4 the branch should show 1 commit ahead of merge-base. After Task 3 rebase, it will be 1 commit ahead of `origin/main`.

---

## Task 3: Rebase onto current main

**Files:** Git history; potentially source files during conflict resolution.

- [ ] **Step 1: Initiate rebase**

Run:
```bash
git rebase origin/main
```

Two possible outcomes:

(a) **Clean rebase**: rebase completes silently. Skip to Step 4.

(b) **Conflict**: rebase pauses with a conflict. Continue to Step 2.

- [ ] **Step 2: Resolve conflicts (only if Step 1 produced a conflict)**

For each conflicted file:

1. Run `git status` to identify the file.
2. Open the file and resolve markers per the policy:
   - **Default**: take main's version (`<<<<<<< HEAD` block, the upper one in `git rerere` order)
   - **Exception 1**: bats files in `tests/bats/bsdtar_compat/` and `tests/bats/gnutar_compat/` — preserve branch-side axis additions while merging in any main-side improvements (e.g., new helper assertions). When in doubt about test semantics, prefer the branch side and add a `# TODO post-rebase: reconcile with main change <hash>` comment near the conflict point.
   - **Exception 2**: `xtask/Cargo.toml` and `xtask/src/main.rs` — preserve branch-side additions (the `bsdtar-compat` subcommand registration), merge with main-side updates to xtask itself.
3. Run `git add <file>` for each resolved file.
4. Run `git rebase --continue`.

- [ ] **Step 3: Halt if conflict count is unmanageable**

If a single rebase --continue produces another conflict and the cumulative count of conflict resolution sessions exceeds **5**, stop and consult the user. Run:
```bash
git rebase --abort
```
to revert to the pre-rebase state. The user will decide whether to redesign the rebase strategy.

- [ ] **Step 4: Verify rebase completion**

Run:
```bash
git log --oneline origin/main..HEAD
```

Expected: exactly 1 commit, the squashed `:sparkles: Add bsdtar oracle compatibility testing framework`. The line above (which would be `origin/main`'s tip) is not shown by this command since it filters from main exclusively.

Run:
```bash
git status
```

Expected: `On branch ci/bsdtar-compat-labels`, `Your branch and 'origin/ci/bsdtar-compat-labels' have diverged`. This is the expected divergence — local has been rebased, origin still points to `ce587cec`.

---

## Task 4: Replace `experimental stdio` alias with `compat bsdtar`

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs` (2 occurrences, post-rebase line numbers may differ from 765/774; identify by content)

- [ ] **Step 1: Locate the lines to change**

Run:
```bash
grep -n 'experimental.*stdio' xtask/src/bsdtar_compat.rs
```

Expected: 2 matches, both inside `run_scenario`, looking like:
```
765:            .args(["experimental", "stdio", "--unstable"])
774:            .args(["experimental", "stdio", "--unstable"])
```

Line numbers may have shifted post-rebase. Note both line numbers from your output.

- [ ] **Step 2: Replace both occurrences**

Use sed for atomic replacement:

```bash
sed -i.bak 's|"experimental", "stdio", "--unstable"|"compat", "bsdtar", "--unstable"|g' xtask/src/bsdtar_compat.rs
rm xtask/src/bsdtar_compat.rs.bak
```

(`-i.bak` is the BSD-portable form for in-place edit with backup; the backup is removed afterward.)

- [ ] **Step 3: Verify replacement**

Run:
```bash
grep -n 'experimental.*stdio' xtask/src/bsdtar_compat.rs
grep -n 'compat", "bsdtar", "--unstable' xtask/src/bsdtar_compat.rs
```

Expected:
- First grep: **0 matches** (empty output, exit code 1)
- Second grep: **2 matches** at the same line numbers as the original `experimental stdio` lines

- [ ] **Step 4: Commit the alias replacement**

Run:
```bash
git add xtask/src/bsdtar_compat.rs
git commit -m ":recycle: Switch xtask oracle to compat bsdtar from deprecated experimental stdio alias"
```

Expected: commit succeeds. Verify with:
```bash
git log origin/main..HEAD --oneline
```
Expected: 2 commits (the squash + the alias replacement).

---

## Task 5: Local build sanity

**Files:** None modified.

- [ ] **Step 1: Build xtask**

Run:
```bash
cargo build -p xtask 2>&1 | tail -10
```

Expected: `Finished ...`. If build fails:
- Read the error.
- If it relates to main-side API changes the rebased framework hasn't adapted to (e.g., a struct field renamed), apply the minimal fix to `xtask/src/bsdtar_compat.rs`, then `git add xtask/src/bsdtar_compat.rs && git commit --amend --no-edit` to fold the fix into the alias replacement commit.
- If the error is broader than 5 lines of fix, halt and consult the user — the rebase may have introduced a non-trivial integration gap that warrants its own commit and possibly its own brainstorming.

- [ ] **Step 2: (Optional) Smoke-test xtask CLI surface**

Run:
```bash
cargo run -p xtask -- bsdtar-compat --help 2>&1 | head -20
```

Expected: `Usage: xtask bsdtar-compat ...` etc. This confirms the binary loads and the subcommand is registered. **Do NOT run the full compat suite locally** — that's covered by CI in Task 7.

---

## Task 6: Force-push the rebased branch

**Files:** None modified.

- [ ] **Step 1: Confirm pre-flight (re-verify no open PR)**

Run:
```bash
gh pr list --head ci/bsdtar-compat-labels --json number,title
```

Expected: `[]`. If non-empty, **do not push** — abort and consult the user.

- [ ] **Step 2: Push with `--force-with-lease`**

Run:
```bash
git push --force-with-lease origin ci/bsdtar-compat-labels
```

Expected output ends with:
```
+ ce587cec...<new-hash>  ci/bsdtar-compat-labels -> ci/bsdtar-compat-labels (forced update)
```

If `--force-with-lease` rejects the push (because remote has new commits), someone else has pushed since the last `fetch`. Stop, run `git fetch origin ci/bsdtar-compat-labels`, examine the new remote commit (`git log -1 origin/ci/bsdtar-compat-labels`), and consult the user.

- [ ] **Step 3: Verify origin updated**

Run:
```bash
git fetch origin ci/bsdtar-compat-labels
git log --oneline origin/ci/bsdtar-compat-labels -3
```

Expected: 3 lines showing (top-to-bottom) the alias `:recycle:`, the squash `:sparkles:`, and the latest origin/main commit.

---

## Task 7: CI observation and fail axis tracker issue

**Files:** None modified.

- [ ] **Step 1: Wait for CI to start, then list runs**

Run:
```bash
sleep 10  # allow CI to register the push
gh run list --branch ci/bsdtar-compat-labels --workflow bsdtar-compat.yml --limit 5
```

Expected: at least one new run in `queued` or `in_progress` state for the latest commit.

Identify the run ID for the most recent `bsdtar compatibility` workflow run for the new HEAD hash. Note it as `$RUN_ID`.

- [ ] **Step 2: Watch the run to completion (allow failure)**

Run:
```bash
gh run watch "$RUN_ID" --exit-status || true
```

The `|| true` is intentional: per the design (option C), Stage 1 does **not** require all axes to pass. Continue regardless of exit code.

- [ ] **Step 3: Capture failed axes**

Run:
```bash
gh run view "$RUN_ID" --log-failed > /tmp/post-rebase-fail.log
grep '^\[FAIL\]' /tmp/post-rebase-fail.log | sort -u > /tmp/fail-axis.txt
wc -l /tmp/fail-axis.txt
```

If `/tmp/fail-axis.txt` is empty (0 lines), all axes passed. Stage 1 is then unconditionally complete; skip to Step 5 and create a tracker issue noting "0 failures observed at rebase time".

- [ ] **Step 4: Build the tracker issue body**

Run (single command, generates the issue body to `/tmp/tracker-body.md`):
```bash
{
  echo '## Context'
  echo ''
  echo 'After rebase of `ci/bsdtar-compat-labels` onto main (Stage 1 completion), differential testing produced the following failed axes. These failures are out of scope for Stage 1 per the design — see `docs/plans/2026-04-26-Stage1-rebase-design.md` (option C, Stage 1 acceptance criteria).'
  echo ''
  echo '## Failed axes'
  echo ''
  echo '| # | Scenario | Notes |'
  echo '|---|---|---|'
  awk 'BEGIN{i=1} /^\[FAIL\]/ {scen=$2; print "| " i " | `" scen "` | analysis pending |"; i++}' /tmp/fail-axis.txt
  echo ''
  echo '## Follow-up'
  echo ''
  echo 'Each row should be analyzed and either:'
  echo '- escalated to a per-axis fix issue, or'
  echo '- documented as a known PNA / bsdtar divergence with rationale (referencing libarchive source).'
  echo ''
  echo 'Investigation work belongs to subsequent stages or independent issues, NOT to Stage 1.'
} > /tmp/tracker-body.md

cat /tmp/tracker-body.md | head -10
```

Expected: a markdown body with a table populated from the fail axes.

- [ ] **Step 5: Create the GitHub tracker issue**

Run:
```bash
gh issue create \
  --title "bsdtar-compat post-rebase fail axis tracker" \
  --body-file /tmp/tracker-body.md
```

Expected: the command prints the new issue URL. Save it for the next step.

- [ ] **Step 6: Mark Stage 1 complete by updating the design doc**

Append to `docs/plans/2026-04-26-Stage1-rebase-design.md`:

```bash
ISSUE_URL=$(gh issue list --search "bsdtar-compat post-rebase fail axis tracker" --json url --limit 1 -q '.[0].url')
RUN_URL=$(gh run view "$RUN_ID" --json url -q '.url')

cat >> docs/plans/2026-04-26-Stage1-rebase-design.md <<EOF

## Stage 1 Completion Record

- Date: $(date -u +%Y-%m-%d)
- Final HEAD: $(git log -1 --format=%h)
- CI run: $RUN_URL
- Fail axis tracker issue: $ISSUE_URL
- Status: **categorized complete** per design option (C)
EOF

git add docs/plans/2026-04-26-Stage1-rebase-design.md
git commit -m ":memo: Record Stage 1 rebase completion"
git push origin ci/bsdtar-compat-labels
```

Expected: 1 additional commit on the branch, push succeeds (no force-with-lease needed since this is a fast-forward).

---

## Self-Review

| Check | Result |
|---|---|
| **Spec coverage** | Each Acceptance Criteria row maps: Rebase → Task 3, Squash → Task 2, Alias replacement → Task 4, Build → Task 5, Push → Task 6, CI trigger → Task 7 Step 1, Fail axis tracker → Task 7 Step 5. ✅ |
| **Placeholder scan** | `<new-hash>`, `$MERGE_BASE`, `$RUN_ID`, `$ISSUE_URL`, `$RUN_URL` are runtime-fill variables within explicit shell substitution contexts (not undefined steps). No `TBD`/`TODO` exists. ✅ |
| **Type/method consistency** | Variable names (`MERGE_BASE`, `RUN_ID`, `ISSUE_URL`, `RUN_URL`) used consistently across tasks. Branch name `ci/bsdtar-compat-labels` and remote `origin` referenced uniformly. ✅ |
| **Frequent commits** | Task 2 (squash), Task 4 (alias), Task 7 Step 6 (completion record) each commit. Task 3 (rebase) and Task 5 (build) do not commit per their nature. Task 6 pushes existing commits. Acceptable for git-surgery work. ✅ |
| **TDD note** | This plan is git-history surgery + binary replacement, not test-driven feature development. The TDD pattern (failing test → impl → passing test) does not apply structurally. The CI run in Task 7 is the integration verification step. ✅ |
