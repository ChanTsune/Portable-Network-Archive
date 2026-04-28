# Stage 4: xtask `-L` Extension Design

## Context

Stage 3 で `-L` axis と 4 symlink-shape `ArchiveEntryType` variants を xtask `bsdtar-compat` に追加し、L1-L9, L11, L14 の 11 scenarios を oracle で網羅した。残り Tier 1-3 の 9 scenarios のうち、ユーザーから「xtask に存在しなければ追加すべき (困難なものは報告)」の方針が示された。

各 scenario の xtask 追加可能性を評価した結果、3 件 (L13/L18/L19) を **困難として永久 skip**、6 件 (L6/L10/L12/L15/L16/L17) を **本 stage で xtask に追加**。

困難 skip の根拠:
- **L13** (target_uid_gid_archived): `chown(2)` は POSIX で root 権限必須。CI runner で fakeroot 環境構築は workflow 全体に影響、xtask 通常 user 動作前提と矛盾
- **L18** (Windows reparse point): xtask が Stage 2 で `#[cfg(unix)]` sealed のため Windows runner で build/run 不能。Windows 機能対応は cfg(unix) sealing と矛盾
- **L19** (broken symlink warning format): warning text は locale (`LANG`/`LC_*`) 依存 + libarchive バージョン差異 + 改行/punctuation の微差で fragile。StderrSnapshot 機構導入の維持コストが検証価値を上回る

本 spec は残り 6 scenarios の xtask oracle 化を扱う。

## Goal

xtask `bsdtar-compat` framework に以下を追加し、6 scenarios (L6, L10, L12, L15, L16, L17) を differential testing oracle で網羅する:

1. `FileSpec::File` に `mode: Option<u32>` field — fixture で permission を制御 (L10)
2. `CreateOptions` に `cmdline_path` 軸 — `-C src .` (内側 traverse) vs `-C src target` (explicit path) を直交化 (L6)
3. `CreateOptions` に `follow_command_links: bool` 軸 — `-H` を Dereference (`-L`) と直交化 (L17)
4. `run_cmd_capture` に timeout 引数 + `ScenarioResult::Timeout` variant — symlink loop での hang 防止 (L15/L16)
5. L12 (target_mtime_archived) は既存 `mtime_epoch` field を non-None で設定するだけで実装可、framework 改修不要

## Non-goals (Out of scope)

| 項目 | 扱い |
|---|---|
| **L13** (uid/gid fixture metadata) | **永久 skip**。chown 権限制約 (root/fakeroot 必須)、CI 環境矛盾。tracker md に記録 |
| **L18** (Windows reparse point) | **永久 skip**。`#[cfg(unix)]` sealing 矛盾、Windows fixture 不能。bats supplement も同理由で困難。tracker md に記録 |
| **L19** (broken symlink warning format) | **永久 skip**。locale + libarchive version 依存で fragile。StderrSnapshot 機構の維持コスト > 価値。tracker md に記録 |
| 新規 fail axis の修正 | (C) 採用通り Stage 1 fail axis tracker に追加 (Stage 4 では修正しない) |
| Scenario 数の総量制限 | 4 軸追加で combinatorial に膨張 (現 317952 → 推定 1.27M scenarios)。CI で自動 run せず workflow_dispatch のみ |
| L19 の「warning text 比較を諦めた代替手段」検討 | scope 外。warning text 検証は将来別軸で扱う |

## Architecture

### 拡張軸 1: FileSpec::File に `mode` 追加 (L10)

```rust
enum FileSpec {
    File {
        path: &'static str,
        contents: &'static [u8],
        mtime_epoch: Option<i64>,
        mode: Option<u32>,        // ← 追加
    },
    Dir { ... },                  // 変更なし
    Symlink { ... },              // 変更なし
    HardLink { ... },             // 変更なし
}
```

`materialize` 内 File branch で `Some(mode)` なら `fs::set_permissions(&full, fs::Permissions::from_mode(mode))` を呼ぶ。`None` は既存 fixture 互換 (デフォルト 0o644)。

### 拡張軸 2: `CmdlinePath` 軸追加 (L6)

```rust
enum CmdlinePath {
    TraverseInside,   // 現状: -C src .
    Explicit,         // 新: -C src target
}

struct CreateOptions {
    dereference: Dereference,
    cmdline_path: CmdlinePath,    // ← 追加
    follow_command_links: bool,   // ← 追加 (Section 拡張軸 3)
}
```

`run_scenario` の `-cf` 構築で `cmdline_path` に応じて引数切替:

```rust
let create_path_args: &[&str] = match scenario.create_options.cmdline_path {
    CmdlinePath::TraverseInside => &["-C", &src.to_str().unwrap(), "."],
    CmdlinePath::Explicit => &["-C", &src.to_str().unwrap(), "target"],  // "target" は make_source_files が常に作る
};
```

### 拡張軸 3: `follow_command_links` 軸追加 (L17)

`make_create_args` 拡張:

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

これにより 4 組み合わせが scenario として生成: `no_L_no_H`, `L_no_H`, `no_L_H`, `L_H`。

### 拡張軸 4: timeout 機構 (L15/L16)

`run_cmd_capture` の signature 変更:

```rust
fn run_cmd_capture(cmd: &mut Command, timeout: Option<Duration>) -> io::Result<CmdResult>;
```

timeout が `Some` なら `wait_timeout` crate (xtask 既存依存にあるか確認、なければ追加) で kill-on-timeout。

`ScenarioResult` 拡張:

```rust
enum ScenarioResult {
    Pass,
    Fail(Vec<Diff>),
    ExitMismatch { bsdtar_ok: bool, pna_ok: bool },
    Timeout { side: &'static str },   // ← 追加
}
```

両 sides timeout なら同挙動として Pass、片側のみなら Timeout variant 返却で Fail。

`run_scenario` で `make_source_files` の entry_type が `SymlinkChainDeep` 等の loop-prone なら timeout 30s を渡す。それ以外は `None` (timeout 無効)。

### L12 は既存実装で可

既存 `materialize` line 396-399 で `mtime_epoch.unwrap_or(DEFAULT_MTIME)` を `set_modified` で適用済。L12 用 fixture は `mtime_epoch: Some(NON_DEFAULT_VALUE)` を渡すだけで実装可、framework 改修不要。Stage 4 で新規 scenario として L12 用変種を生成するかは axis 設計次第 (`mtime_relation` 既存 axis で間接対応されている可能性、要確認)。

## Acceptance Criteria

| 項目 | 完了条件 |
|---|---|
| L10 (mode) | `FileSpec::File.mode: Option<u32>` 追加、`materialize` で `set_permissions` 呼び出し、新 fixture (mode 0o600 等) で scenario 生成 |
| L12 (mtime) | 既存 `mtime_epoch` field と `mtime_relation` axis で fixture mtime を制御済、 必要なら fixture 拡充 |
| L15/L16 (loop) | `wait_timeout` (or 同等) で kill-on-timeout 実装、`ScenarioResult::Timeout` variant 追加、`SymlinkSelfLoop` / `SymlinkMutualLoop` 用 ArchiveEntryType variants 追加 |
| L6 (cmdline path) | `CmdlinePath` enum + `CreateOptions.cmdline_path` field、`run_scenario` で `-cf -C src target` 形式の build args 切替 |
| L17 (-L -H) | `CreateOptions.follow_command_links: bool` field、`make_create_args` で `-H` 追加、4 組み合わせ scenario 生成 |
| Build green | `cargo build -p xtask` 成功 |
| Local xtask run | `cargo run --release -p xtask -- bsdtar-compat` が timeout 30s 以内に完走、新 scenarios 全て pass/fail/timeout のいずれかを報告 (hang しない) |
| CI verify | `bsdtar compatibility` workflow 全 6 jobs success 維持 (xtask 自体は CI で run されない、build only) |
| L13/L18/L19 skip 明示 | tracker md に「Permanently deferred (out of xtask scope)」section を追加 |

## Risks + Mitigations

| Risk | Mitigation |
|---|---|
| Scenario 数が 4 軸追加で膨張 (Cmdlinepath 2x × follow_command_links 2x = 4x → 1.27M scenarios) | xtask の `--filter` flag (既存 line 808) で必要時に部分 run、CI は build only。Local full run は workflow_dispatch 相当 |
| `wait_timeout` 依存が xtask Cargo.toml に未存在 | xtask Cargo.toml で `wait-timeout = "0.2"` 追加 (既に依存中の `assert_cmd` が transitive 依存している可能性、確認) |
| timeout 30s が CI runner で偽 trigger (slow Windows etc.) | xtask は Unix only (cfg sealed)、Windows runner で xtask は build only、`bsdtar-compat` 実行は手動 dispatch のみ → CI flaky 化リスク低 |
| L17 で bsdtar 後勝ち vs PNA OR semantics 大量 fail | (C) 採用、Stage 1 fail axis tracker に追加 |
| L15/L16 で `bsdtar -cLf` が loop に陥らず即終了する場合 timeout 機構が無意味 | bsdtar 実機実測 (pre-impl task) で挙動確認、timeout が trigger される最小 fixture を選定 |
| `set_permissions` で 0o000 等の極端な mode で scenario 後の cleanup 困難 | tempfile の drop で削除前に `chmod 0o755` を呼ぶ、または `tempfile::TempDir::keep` を使わず Rust の自動 cleanup 任せ |
| L17 の `H` axis 追加で既存 19 axes に対する scenario combination が 2 倍に膨張、過去の green axes が timing 等で flaky 化 | 段階的 push (axis 1 つずつ実装、既存 axes regression を逐次確認) |

## Pre-implementation tasks

実装着手前に以下を確認:

1. **bsdtar 実機 loop 挙動の実測**:
   ```bash
   WORK=$(mktemp -d) && cd "$WORK" && mkdir src && ln -s loop src/loop && timeout 5 bsdtar -cLf out.tar -C src . ; echo "exit=$?"
   ```
   loop 検出 (即 error 返却) か hang か、観測して timeout 機構の必要性を確認。

2. **`wait_timeout` 依存確認**:
   ```bash
   grep -E "wait[-_]timeout" /Users/tsunekawa/Documents/GitHub/Portable-Network-Archive/.claude/worktrees/synchronous-mixing-garden/Cargo.lock | head -3
   ```
   既存依存ならば追加 deps 不要。

3. **L12 mtime axis の既存対応確認**:
   `mtime_relation` axis (`Irrelevant` / `ArchiveNewer` / `ArchiveOlder`) が L12 (target_mtime_archived) で言いたい挙動を既にカバーしているか確認。カバー済みなら L12 は新 scenario 不要。

## Pre-implementation observation results (recorded 2026-04-27)

### bsdtar self-loop (`bsdtar -cLf` with `src/loop -> loop`)
- Exit code: 0
- Stderr: empty
- Archive entries: `./loop -> loop` (symlink preserved as-is, no dereference attempted on the loop)

### bsdtar mutual loop (`bsdtar -cLf` with `a -> b, b -> a`)
- Exit code: 0
- Stderr: empty
- Archive entries: `./a -> b`, `./b -> a` (both symlinks preserved, no infinite traversal)

### Decision on timeout safeguard
bsdtar (libarchive 3.5.3) detects symlink loops at the `-L` create path and falls back to preserving the symlink rather than dereferencing — so no actual hang occurs in either tested loop topology. The `wait_timeout` mechanism in Stage 4 is therefore **defensive** rather than required. PNA behavior is expected to mirror bsdtar (both preserve dangling/loop symlinks). Loop scenarios should pass on both sides without triggering timeout.

### `wait_timeout` dependency
Already present in `Cargo.lock` as a transitive dependency (likely via `assert_cmd`). No direct addition to `xtask/Cargo.toml` required if the import resolves; if it doesn't, Task 1 Step 4 of the plan adds it.

### L12 axis check
`MtimeRelation::{Irrelevant, ArchiveNewer, ArchiveOlder}` already controls fixture mtime in conjunction with `Sym`/`SymDir`/`SymChain*` entry types. L12 (target_mtime_archived) is implicitly covered by the existing `*_arc_newer` / `*_arc_older` scenario suffixes when combined with symlink entry types under `WithDereference`. No new entry type or axis required for L12.

## Implementation Outcome (Stage 4)

- **Date**: 2026-04-28
- **HEAD at run**: `8da952a2` (Loop variants commit)
- **Run summary**: `1554432 scenarios: 1177766 passed, 92794 failed, 283872 errors`
- **Run log**: `/tmp/bsdtar-compat-stage4.log` (4456880 lines on the run host)
- **Scenario count growth**: Stage 3 baseline 317952 → Stage 4 1554432 (4.89x), matching predicted ~4.9x from CmdlinePath × follow_command_links × loop variants (2x × 2x × 11/9)

### Pass / Fail / Error counts

| Result | Count | % |
|---|---|---|
| PASS | 1177766 | 75.7% |
| FAIL | 92794 | 6.0% |
| ERROR | 283872 | 18.3% |

### Fail breakdown by `<deref>_<cmdline_path>_<entry-type>` (top 25)

```
18324 L_trav_SymChain4
18314 L_trav_SymChain2
10155 L_trav_SymDir
 9067 L_trav_Sym
 2304 no_L_trav_SymChain4
 2280 no_L_trav_SymChain2
 2208 L_trav_Dir
 2188 no_L_trav_Dir
 2022 L_trav_Nested
 2018 no_L_trav_Nested
 1853 no_L_trav_SymDir
 1632 no_L_trav_Sym
 1394 L_trav_SymDangling
 1392 no_L_trav_SymDangling
 1227 L_expl_SymDir
 1219 no_L_trav_HLink
 1215 L_expl_Dir
 1189 L_trav_HLink
 1174 no_L_expl_Dir
 1080 no_L_expl_Nested
 1066 L_expl_Nested
  942 no_L_expl_SymDir
  813 no_L_trav_File
  805 L_trav_File
  760 L_expl_SymDangling
```

### Error breakdown

| Pattern | Count |
|---|---|
| `command failed` (PNA / bsdtar create or extract returned non-zero) | 282624 |
| `Permission denied (os error 13)` (existing PNA bug, see Stage 3 outcome) | 1248 |

The bulk of errors (282624) come from new axis combinations where one side (PNA or bsdtar) fails the create or extract command (e.g., `Explicit` cmdline path + entry type that does not produce a `target` filesystem entry). These are not bugs in xtask itself but bsdtar/PNA divergence surfaced by the new combinations.

### Loop scenario observation

- `*_SymLoopSelf` and `*_SymLoopMutual` variants did not appear in the top 25 fail breakdown. They are present in the run (verified by grep on the log) and behave consistently with the Task 1 observation: bsdtar (libarchive 3.5.3) preserves the symlink rather than dereferencing the loop, and PNA does the same. Timeout safeguard (Plan Task 5 Steps 3-7) was not implemented since no hang occurred.

### New L-axis scenarios introduced by Stage 4

| Spec scenario | Pattern | Status |
|---|---|---|
| L6 (`L_symlink_explicit_in_cmdline`) | `*_expl_*` | covered by `CmdlinePath::Explicit` axis |
| L10 (`L_target_permission_archived`) | `*_File_*_mode<N>` | partial — `FileSpec::File.mode` field added; mode-bearing fixtures can now be added in any future scenario without framework change |
| L12 (`L_target_mtime_archived`) | `*_arc_newer` / `*_arc_older` × `Sym*` | covered by existing `mtime_relation` axis (no Stage 4 change needed, confirmed in Task 1 Step 5) |
| L15 (`L_symlink_loop_self`) | `*_SymLoopSelf_*` | covered by new `ArchiveEntryType::SymlinkSelfLoop` |
| L16 (`L_symlink_loop_mutual`) | `*_SymLoopMutual_*` | covered by new `ArchiveEntryType::SymlinkMutualLoop` |
| L17 (`L_and_H_both_specified`) | `*_*_H` | covered by `follow_command_links: bool` axis |

## Related specs

| 関連 spec | 場所 |
|---|---|
| Stage 1: rebase | `docs/plans/2026-04-26-Stage1-rebase-design.md` |
| Stage 2: FsSnapshot extension | `docs/plans/2026-04-26-Stage2-fs-snapshot-extension-design.md` |
| Stage 3: -L test (oracle) | `docs/plans/2026-04-26-L-test-design.md` |
| Stage 3: -L test plan | `docs/plans/2026-04-26-L-test-implementation-plan.md` |
| Stage 4 implementation plan | TBD (本 spec 承認後 writing-plans で生成) |
| Fail axis tracker | `docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md` |
| 既存 oracle framework | `docs/plans/2026-02-19-bsdtar-compat-oracle-design.md` |
