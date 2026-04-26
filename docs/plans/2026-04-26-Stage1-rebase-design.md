# Stage 1: `ci/bsdtar-compat-labels` Rebase Design

## Context

`ci/bsdtar-compat-labels` には bsdtar oracle compatibility testing framework (`xtask bsdtar-compat`) が構築されている (24 commits ahead of `main` 時点)。一方 `main` は 10+ commits ahead でき、特に以下の変更が反映済:

- `experimental stdio` → `compat bsdtar` への subcommand rename (deprecated alias 化、warning 出力)
- broken pipe handling (`is_broken_pipe()` in `cli/src/main.rs`)
- libpna error message conventions
- multipart archive 関連の error message 改善
- その他多数の改修

`-L` / `--dereference` 検証 (Stage 3) およびその FsSnapshot 拡張 (Stage 2) の前提として、framework が現 `main` 上で動作する状態が必要。本 spec はこの rebase 作業を扱う。

## Goal

`ci/bsdtar-compat-labels` を `main` 上に rebase し、xtask の `experimental stdio` hardcode 参照を `compat bsdtar` に置換する。最終 HEAD は **`main` から linear に派生する 2 commits**:

1. `:sparkles: Add bsdtar oracle compatibility testing framework` (元 24 commits を squash)
2. `:recycle: Switch xtask oracle to compat bsdtar from deprecated experimental stdio alias`

## Non-goals (Out of scope)

| 項目 | 扱い |
|---|---|
| **Fail axis 全数の修正** | (C) 採用により別 issue 追跡。Stage 1 は categorized 完了で終了し、PNA の挙動 regression は本 stage の責務外 |
| libarchive build cache の workflow 改修 | `ci/bsdtar-compat-enable-cache` branch にある別作業、本 stage 範疇外 |
| FsSnapshot の metadata 拡張 (mode/mtime/uid/gid) | Stage 2 (別 brainstorming) |
| `-L` axis 追加 | Stage 3 (別 plan、本 brainstorming で生成済) |
| bats 補完 | Stage 4 (別 brainstorming) |
| 22 個別 commit の history 保持 | Approach 1 ではなく **Approach 2 (squash)** を採用したため、framework と doc の全 24 commits を 1 commit に集約 |

## Architecture (commit 構造)

```
origin/main (current HEAD: e.g., d85e4eca)
   ↓
[1] :sparkles: Add bsdtar oracle compatibility testing framework
    (元 24 commits の全 diff を含む 1 commit)
   ↓
[2] :recycle: Switch xtask oracle to compat bsdtar from deprecated experimental stdio alias
    (xtask/src/bsdtar_compat.rs の 2 箇所のみ修正)
```

最終 `ci/bsdtar-compat-labels` = `origin/main` + 2 commits ahead。

## Steps

### Step 1: 24 commits を 1 commit に squash

```bash
git switch ci/bsdtar-compat-labels
git fetch origin main
MERGE_BASE=$(git merge-base HEAD origin/main)
git reset --soft "$MERGE_BASE"
git commit -m ":sparkles: Add bsdtar oracle compatibility testing framework"
```

期待: HEAD が merge-base + 1 commit (24 commits 全変更を含む単一 commit)。

### Step 2: `main` に rebase

```bash
git rebase origin/main
```

conflict 解決方針:
- **デフォルト: main 側採用** (main マージ済の決定を尊重)
- **branch 側採用の例外**:
  - `docs/plans/2026-02-19-bsdtar-compat-oracle-design.md` 等 main に存在しない doc (= branch 側のみ存在 → conflict 不発生、念のため明示)
  - branch が axis 追加した bats test (`test_basic.bats`, `test_option_*.bats` 等) で main も同じ test を変更している場合 → axis 追加意図を保ちつつ main の改修を取り込む手動 merge
- 解決後: `git add <file>` + `git rebase --continue`

### Step 3: alias 置換 commit

```diff
# xtask/src/bsdtar_compat.rs:765, 774 (rebase 後の line)
-            .args(["experimental", "stdio", "--unstable"])
+            .args(["compat", "bsdtar", "--unstable"])
```

```bash
git add xtask/src/bsdtar_compat.rs
git commit -m ":recycle: Switch xtask oracle to compat bsdtar from deprecated experimental stdio alias"
```

### Step 4: Local sanity (build only)

```bash
cargo build -p xtask
```

期待: 成功。失敗時は alias 置換 + main 由来 API 変更を確認、必要なら `make_create_args` 等の signature 不整合を調整 (本 stage 内修正、追加 commit が必要なら別途)。

`cargo run -p xtask -- bsdtar-compat` の **実行はローカル必須としない**。fail axis 追跡は CI 上で行う方が均質。

### Step 5: Force-with-lease push

```bash
git push --force-with-lease origin ci/bsdtar-compat-labels
```

事前確認: `gh pr list --head ci/bsdtar-compat-labels` で open PR 0 件確認。

### Step 6: CI 観察 + fail axis 追跡

```bash
gh run list --branch ci/bsdtar-compat-labels --limit 5
gh run watch <run-id> --exit-status || true   # fail でも続行 (categorized 完了)
gh run view <run-id> --log-failed > /tmp/post-rebase-fail.log
grep '^\[FAIL\]' /tmp/post-rebase-fail.log > /tmp/fail-axis.txt
```

fail axis を **bulk 1 issue** で表羅列:

```bash
gh issue create \
  --title "bsdtar-compat post-rebase fail axis tracker" \
  --body "$(cat <<EOF
## Context
After rebase of \`ci/bsdtar-compat-labels\` onto main, the following axes failed in differential testing.
These failures are out of scope for Stage 1 per the design (option (C) chosen for Stage 1 acceptance).

## Failed axes

| Scenario | Failure type | Notes |
|---|---|---|
$(cat /tmp/fail-axis.txt | sed 's|^|| | TBD | |')

## Follow-up
Each row should be analyzed and either:
- escalated to a per-axis fix issue, or
- documented as known PNA divergence with rationale.
EOF
)"
```

個別 issue split は別 stage の作業者の裁量。

## Acceptance Criteria

| 項目 | 完了条件 |
|---|---|
| Rebase | `git rebase origin/main` 完了、HEAD が main 由来 + 2 commits ahead |
| Squash | `git log origin/main..HEAD --oneline` で 2 行のみ表示 |
| Alias replacement | `grep -rn 'experimental.*stdio' xtask/src/` が **0 hits** |
| Build | `cargo build -p xtask` 成功 |
| Push | `git push --force-with-lease` 成功、`gh pr list --head ci/bsdtar-compat-labels` で open PR との衝突なし |
| CI trigger | `gh run list` で新 HEAD 用 run が queue/in_progress/completed のいずれか |
| Fail axis 追跡 | bulk tracker issue 1 件が GitHub に open、表に fail scenarios 羅列 |

**Stage 1 完了 = 上記 7 項目すべて充足**。CI run の all-pass は完了条件ではない (Stage 1 範疇外、(C) 採用)。

## Risks + Mitigations

| Risk | Mitigation |
|---|---|
| squash 1 commit の rebase で大規模 conflict | conflict は 1 セッションで解決、main 採用デフォルト。conflict 解決中に branch 側意図 (axis 追加) を保つ判断が必要なら commit message を読んで判断、判断つかない時は ユーザー確認 |
| force-with-lease push で連携破壊 | 事前に `gh pr list --head ci/bsdtar-compat-labels` で open PR 0 件確認 |
| CI runner で libarchive build 長時間化 | 既存 workflow の cache 設定 (`ci/bsdtar-compat-enable-cache` branch) は **本 stage 範疇外**、別 issue に記録 |
| fail axis 数が膨大 (>>100) で issue 化が non-trivial | bulk 1 tracker issue で表羅列、個別 split は別 stage 作業者の裁量 |
| alias 置換後の build エラー (main 由来 API 変更) | Step 4 で検出、修正は alias 置換 commit に追加 (separate commit する場合のみ別 commit で) |
| `compat bsdtar` の `--unstable` 要求が変わっている | main の `bsdtar.rs:590` で `requires = "unstable"` 確認済 (既存と同じ)、変更なし |
| squash 後の commit message が情報量不足 | commit message は短く、framework の詳細 (axis list 等) は `docs/plans/2026-02-19-bsdtar-compat-oracle*.md` で補完 |
| design/plan doc が squash で 1 commit に埋もれる | doc file 自体が remain、git blame で 1 commit を指すが内容は doc 内で完結 |

## Pre-implementation tasks

なし (本 stage は実装そのものが rebase 作業、事前調査は本 spec で完了)。

## Related specs

| 関連 spec | 場所 |
|---|---|
| Stage 2: FsSnapshot extension design | TBD (別 brainstorming) |
| Stage 3: -L test design | `docs/plans/2026-04-26-L-test-design.md` (本 branch) |
| Stage 3: -L test implementation plan | `docs/plans/2026-04-26-L-test-implementation-plan.md` (本 branch) |
| Stage 4: bats supplement | TBD (別 brainstorming) |
| 既存 oracle framework design (rebased 後) | `docs/plans/2026-02-19-bsdtar-compat-oracle-design.md` |
| 既存 oracle framework plan (rebased 後) | `docs/plans/2026-02-19-bsdtar-compat-oracle.md` |
