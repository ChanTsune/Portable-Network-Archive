# Stage 2: FsSnapshot Metadata Extension Design

## Context

`xtask/src/bsdtar_compat.rs` の `FsSnapshot` は bsdtar oracle differential testing で extract 後の filesystem 状態を比較する。現状の `FsEntry` は以下の field のみ保持:

```rust
enum FsEntry {
    File { contents: Vec<u8>, mode: u32, mtime_secs: i64 },
    Dir { mode: u32 },
    Symlink { target: PathBuf },
}
```

Stage 3 (`-L` axis) や `--no-same-owner` / `-p` / `--keep-newer-files` 等の既存 axes で **uid/gid/mtime の差** を network 検出する必要があるが、現 snapshot は uid/gid を持たず、Dir には mtime も持たない。bsdtar が POSIX tar header に dir mtime を保存する事実に対し、現 oracle ではこれを verify できない。

本 spec は `FsSnapshot` の field 追加のみを扱う最小改修 design。

## Goal

`FsEntry` に以下を追加:
- `File`: `uid: u32`, `gid: u32`
- `Dir`: `mtime_secs: i64`, `uid: u32`, `gid: u32`

これにより既存 axes (`--no-same-owner`, `-p`, `--keep-newer-files`) と Stage 3 の L10/L12/L13 scenarios が metadata 差を network 観測可能になる。

## Non-goals (Out of scope)

| 項目 | 扱い |
|---|---|
| `FileSpec` への uid/gid 追加 | scope 外。fixture は system uid のまま (chown 権限不要) |
| `materialize()` での chown 呼び出し | 同上、scope 外 |
| Symlink の mode/mtime/uid/gid | OS で扱い不安定 (lchmod/lutimes の portability)、scope 外 |
| Windows fallback | 既存 `xtask/src/bsdtar_compat.rs:3` が `MetadataExt` を unconditional import (Unix only) のため、本 stage も Unix only 前提を継承 |
| 新規 fail axis の修正 | (C) 採用通り、Stage 1 fail axis tracker に追加 (Stage 2 では修正しない) |
| atime/ctime/nlinks 等の他 metadata | 必要性低、scope 拡大避ける |
| Display format の versioning | 既存 axes の expected snapshot に format 変更が影響するが、本 stage 内修正の責務に留める |

## Architecture

### Data Model 変更

```rust
enum FsEntry {
    File {
        contents: Vec<u8>,
        mode: u32,           // 既存
        mtime_secs: i64,     // 既存
        uid: u32,            // ← 追加
        gid: u32,            // ← 追加
    },
    Dir {
        mode: u32,           // 既存
        mtime_secs: i64,     // ← 追加
        uid: u32,            // ← 追加
        gid: u32,            // ← 追加
    },
    Symlink {
        target: PathBuf,     // 変更なし
    },
}
```

### Capture 変更 (`FsSnapshot::walk`)

```rust
// Dir branch
let mode = meta.mode() & 0o7777;
let mtime_secs = meta.mtime();
let uid = meta.uid();
let gid = meta.gid();
entries.insert(rel.clone(), FsEntry::Dir { mode, mtime_secs, uid, gid });

// File branch
let contents = fs::read(&path)?;
let mode = meta.mode() & 0o7777;
let mtime_secs = meta.mtime();
let uid = meta.uid();
let gid = meta.gid();
entries.insert(rel, FsEntry::File { contents, mode, mtime_secs, uid, gid });
```

### Display 変更

`File`: `File("{contents}", mode={mode:04o}, mtime={mtime}, uid={uid}, gid={gid})`
`Dir`: `Dir(mode={mode:04o}, mtime={mtime}, uid={uid}, gid={gid})`
`Symlink`: 変更なし

### 影響範囲

- 直接修正: `xtask/src/bsdtar_compat.rs` 内
  - `FsEntry` enum 定義 (line 554-566)
  - `Display` impl (line 568-587)
  - `FsSnapshot::walk` (line 599-630)
- 間接影響: なし (`FileSpec`, `materialize`, `make_*_args`, scenario generation 等は変更なし)

## Acceptance Criteria

| 項目 | 完了条件 |
|---|---|
| `FsEntry::File` に `uid: u32`, `gid: u32` 追加 | grep で field 確認 |
| `FsEntry::Dir` に `mtime_secs: i64`, `uid: u32`, `gid: u32` 追加 | grep で field 確認 |
| `FsSnapshot::walk` で File/Dir 両 branch で `meta.uid()`, `meta.gid()` 取得 | grep でメソッド呼び出し確認 |
| `FsSnapshot::walk` で Dir branch で `meta.mtime()` 取得 | 同上 |
| `Display` impl が新 field を出力 | unit test or integration test の snapshot 出力で確認 |
| Build green | `cargo build -p xtask` 成功 |
| 既存 axes 実行可能 | `cargo run -p xtask -- bsdtar-compat --help` で smoke test、CI で 1 run trigger 確認 |
| Stage 3 prerequisite 達成 | Stage 3 plan の L10 (mode), L12 (mtime), L13 (uid/gid) scenarios が **理論的に enable** 可能 (= snapshot 比較で metadata 差を観測可能) |
| 新規 fail axis 追跡 | Stage 2 commit 後の CI run で新規に fail する axis があれば、Stage 1 fail axis tracker md (`docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md`) に追記 |

## Risks + Mitigations

| Risk | Mitigation |
|---|---|
| Stage 1 で pass していた axes が Stage 2 で新規に fail (例: `--no-same-owner` で uid 差検出) | (C) 採用通り、Stage 1 tracker md に新規 fail を追加。Stage 2 では修正しない |
| `Display` 出力フォーマット変更で既存 axes の diff message format が変わる | format 変更を Stage 2 commit message に明示。既存 diff format は **test contract ではなく debug 出力** なので、変更しても test contract は破壊されない |
| u32 cast の platform 互換性 | `MetadataExt::uid()` は Linux/macOS で `u32` を直接返す (`std::os::unix::fs::MetadataExt`)、cast 不要 |
| Windows runner で xtask build 失敗 | 既存 `bsdtar_compat.rs:3` の unix-only import で **既に Windows build 不能** の可能性 (Stage 1 後の CI run で Windows job 状況を確認、本 stage で対応しない) |
| dir mtime の比較で nano-second 切り捨て差 | `mtime_secs: i64` (秒精度) で統一、nsec 比較は scope 外 |

## Pre-implementation tasks

なし。本 stage は実装そのものが minimal field 追加、事前調査は本 spec で完了。

## Related specs

| 関連 spec | 場所 |
|---|---|
| Stage 1: rebase design | `docs/plans/2026-04-26-Stage1-rebase-design.md` |
| Stage 1: rebase plan | `docs/plans/2026-04-26-Stage1-rebase-implementation-plan.md` |
| Stage 3: -L test design | `docs/plans/2026-04-26-L-test-design.md` (FileSpec extension の記述あるが、(A) 採用に伴い本 spec で再解釈: FileSpec 拡張は不要、snapshot 拡張のみ) |
| Stage 3: -L test plan | `docs/plans/2026-04-26-L-test-implementation-plan.md` (同上、FileSpec extension 部分は本 spec の方針で読み替え) |
| Stage 4: bats supplement | TBD (別 brainstorming) |
| 既存 oracle framework design | `docs/plans/2026-02-19-bsdtar-compat-oracle-design.md` |
