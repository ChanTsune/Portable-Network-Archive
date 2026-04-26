# `pna compat bsdtar -L / --dereference` Verification Design

## Context

`-L` / `--dereference` の安定化候補性評価で、3 軸基準 (test coverage / 参照実装 parity / edge case) を当てた結果、既存の bats `tests/bats/bsdtar_compat/test_option_L_upper.bats` (Tests 1-4) は **PNA 自身の挙動を test 化しただけで bsdtar 実機との差分検証ではなかった** ことが判明した。具体的には:

- 特殊ファイル (FIFO/device/socket) を `core.rs:756-760` で `Unsupported` error として abort する PNA 挙動が bsdtar parity から逸脱
- dangling symlink を `StoreAs::Symlink` で archive 化する PNA 挙動が bsdtar 実機と一致するか未検証
- `HardlinkResolver::new(follow_links)` で hardlink 検出が `-L` 状態に依存する PNA 設計が bsdtar との独立性を破る可能性
- `-L` + `-H` 同時指定の semantics (bsdtar = 後勝ち / PNA = OR) が CLI 表面で異なる
- xattr/ACL on dereferenced symlink の挙動 未検証

これら検証 gap を埋めるため、`-L` の検証ケースを徹底的に洗い出し、bsdtar 実機との differential testing で parity を保証する design を確立する。

## Goal

`pna compat bsdtar -L / --dereference` の挙動が **bsdtar 本家** (libarchive) と完全互換であることを、以下の 2 系統で network 検証する:

- **A 系統 (oracle)**: `cargo xtask bsdtar-compat` で bsdtar と PNA を両方実行 → filesystem snapshot 比較 → diff report
- **B 系統 (bats)**: oracle が扱えない領域 (loop / option semantics / Windows reparse / error message format) を bats で hard-code expected

成功条件: 3 軸 (test coverage / parity / edge case) 全充足の状態に到達 → `-L` を Tier S (Ready for stabilization) に格上げ可能。

## Non-goals (Out of scope)

本 spec で扱わない (= 別 design として独立する) 項目:

| 項目 | 扱い |
|---|---|
| 特殊ファイル (FIFO / device / socket) | **PNA design decision として scope 外**。`core.rs:756-760` の Unsupported abort は意図された挙動として固定。bsdtar parity を諦める |
| xattr / ACL / file flags が `-L` で target のものに変わるか | `--keep-acl` / `--keep-xattrs` / `--keep-fflags` 各 axis の別 design に委譲 |
| hardlink-to-symlink との `-L` 相互作用 | hardlink axis の別 design |
| `-L --one-file-system` cross-mount | CI 環境で再現困難、別 design or 諦め |
| bsdtar 実機が install できない platform | scenario skip + spec 注記 |

## Architecture (Stage 構造)

`-L` 検証を 4 stage に分割。各 stage は独立 spec / plan / implementation cycle を持つ。

```
Stage 1: rebase ci/bsdtar-compat-labels to main
   ├ conflict resolution
   ├ 既存 axes (--exclude, -U, -p 等) の green 維持確認
   └ Stage 2-4 の前提として安定化

Stage 2: FsSnapshot 拡張 (汎用改修)
   ├ File/Dir variant に metadata field 追加: mode, mtime, uid, gid
   ├ 既存 axes (--no-same-owner, -p, --keep-newer-files) で metadata oracle 比較を有効化
   └ 既存 axes に対する回帰確認

Stage 3: -L axis 追加 (THIS SPEC が対象)
   ├ axis: WithoutDereference / WithDereference の dual run
   ├ 検証 scenario matrix (Tier 1/2/3 で 19 件)
   └ FsSnapshot 拡張 (Stage 2) に依存

Stage 4: bats 補完 (oracle 外)
   ├ symlink loop の error / timeout 挙動
   ├ -L + -H 同時指定の semantics 確定
   ├ Windows reparse point semantics
   └ broken symlink warning format
```

順序依存: Stage 1 → 2 → 3 (3 は 2 に依存)。Stage 4 は 3 と並行可。

## Test Matrix

### 軸の整理

| 軸 | 値 |
|---|---|
| A. Entry kind (source) | regular file / directory / symlink → file / symlink → dir / symlink chain (depth 2+) / dangling symlink / symlink loop / hardlink |
| B. CLI 指定方式 | `-C dir .` (内側 traverse) / 個別 path / 個別 path で symlink 直接指定 |
| C. オプション組合せ | `-L` 単独 / `-L -H` 同時 / `-L --strip-components N` / `-L --exclude pattern` / `-L -s 's,...,...,'` |
| D. 検証項目 | existence / file type / contents / mode / mtime / uid / gid (xattr/ACL/fflags は scope 外) |
| E. Test 配置 | oracle (xtask bsdtar-compat) / bats (oracle 外) |
| F. Platform | Linux / macOS / Windows |

### Tier 1: oracle 必須 (10 件、最低限の network 検証)

| # | Scenario | `-L` | source 構成 | 期待 (oracle 確認) |
|---|---|---|---|---|
| L1 | `L_baseline_no_dereference` | off | symlink → file | 両者: archive 内 symlink、extract 後 symlink |
| L2 | `L_symlink_to_file_dereferenced` | on | symlink → file | 両者: archive 内 regular file、extract 後 regular file (target contents) |
| L3 | `L_symlink_to_dir_dereferenced` | on | symlink → dir (中身あり) | 両者: archive 内 dir + 子 entry、extract 後 dir + contents |
| L4 | `L_dangling_symlink_with_L` | on | symlink → non-existent | **bsdtar 実機実測結果 (macOS, libarchive 3.5.3)**: symlink として保持、exit 0、stderr 空。PNA の `core.rs:780-787` (`StoreAs::Symlink` 保持) と完全 parity 想定 |
| L5 | `L_symlink_chain_2` | on | a → b → file | 両者: archive 内 final target ファイル、extract 後 file |
| L6 | `L_symlink_explicit_in_cmdline` | on | command-line で symlink 直接指定 | **Stage 3 では実装しない** (xtask は `-cf -C src .` 固定、cmdline-explicit path 軸を framework に持たない)。Stage 4 bats supplement で扱う |
| L7 | `L_symlink_in_traversed_dir` | on | `-C dir .` で symlink を traverse | 両者: 同様 dereferenced |
| L8 | `L_dereference_with_strip_components` | on + `--strip-components 1` | symlink → file with prefix path | strip 適用後の名前で archive |
| L9 | `L_dereference_with_exclude` | on + `--exclude '*.bak'` | symlink → file (除外対象 / 非対象) | exclude 適用後 dereferenced |
| L10 | `L_target_permission_archived` | on | symlink → file (mode 0600) | extract 後 mode 0600 (target permission)、symlink 自身でない |

### Tier 2: oracle 推奨 (4 件、エッジ network 検証)

| # | Scenario | `-L` | source 構成 | 期待 |
|---|---|---|---|---|
| L11 | `L_symlink_chain_depth4` | on | a → b → c → d → file | final target のみ archive |
| L12 | `L_target_mtime_archived` | on | symlink → file (mtime 1week ago) | archive 内 mtime = target mtime |
| L13 | `L_target_uid_gid_archived` | on | symlink → file (uid 別) | archive 内 uid/gid = target のもの (要 fakeroot or 動的 uid 取得) |
| L14 | `L_dereference_with_substitution` | on + `-s ',^link,renamed,'` | symlink → file 名前 `link` | substitution 適用 + dereference |

### Tier 3: bats 補完 (5 件、oracle 外)

| # | Scenario | 配置 | 検証内容 |
|---|---|---|---|
| L15 | `L_symlink_loop_self` | bats | `a → a` で `-L` → bsdtar/PNA 双方の error/exit 挙動を hard-code expected |
| L16 | `L_symlink_loop_mutual` | bats | `a → b → a` で `-L` → 同上 |
| L17 | `L_and_H_both_specified` | bats | `-L -H` 同時 → bsdtar 後勝ち vs PNA OR の差を確認、PNA 仕様確定 |
| L18 | `L_windows_reparse_point` | bats (Windows only) | Windows symlink/junction の `-L` 挙動 |
| L19 | `L_broken_symlink_warning_format` | bats | bsdtar の warn 文言 vs PNA の挙動を確認、log format 固定 |

### Scope 外 (本 spec で扱わない、再掲)

- 特殊ファイル (FIFO / device / socket) — PNA design decision 固定済
- hardlink to symlink の `-L` 相互作用 — hardlink axis の別 design
- xattr / ACL / fflags が `-L` で target のものに変わるか — 各 `--keep-*` axis の別 design
- `-L --one-file-system` cross-mount — CI 制約

## Data Model

### FileSpec 拡張 (Stage 2 依存)

Stage 2 で metadata 軸を加えるため、既存 `xtask/src/bsdtar_compat.rs` の `FileSpec` を以下に拡張:

```rust
enum FileSpec {
    File {
        path: &'static str,
        contents: &'static [u8],
        mtime: Option<i64>,
        mode: Option<u32>,    // ← Stage 2 追加
        uid: Option<u32>,     // ← Stage 2 追加
        gid: Option<u32>,     // ← Stage 2 追加
    },
    Dir {
        path: &'static str,
        mode: Option<u32>,    // ← Stage 2 追加
    },
    Symlink { path: &'static str, target: &'static str },
    Hardlink { path: &'static str, target: &'static str },
}
```

`Option<T>` の `None` 既定値は **比較から除外** する semantics。これにより既存 19 axes は `None` のまま動作維持。

### 代表 scenario fixture (4 例、残り 15 件は同パターン)

```rust
// L1: baseline_no_dereference
Scenario {
    name: "L_baseline_no_dereference",
    source_files: &[
        FileSpec::File { path: "src/file.txt", contents: b"hello", mtime: None,
                         mode: None, uid: None, gid: None },
        FileSpec::Symlink { path: "src/link", target: "file.txt" },
    ],
    pre_existing: &[],
    create_options: &[],
    extract_options: &[],
}

// L2: symlink_to_file_dereferenced
Scenario {
    name: "L_symlink_to_file_dereferenced",
    source_files: &[ /* L1 と同 */ ],
    pre_existing: &[],
    create_options: &["-L"],
    extract_options: &[],
}

// L10: target_permission_archived
Scenario {
    name: "L_target_permission_archived",
    source_files: &[
        FileSpec::File { path: "src/file.txt", contents: b"x", mtime: None,
                         mode: Some(0o600), uid: None, gid: None },
        FileSpec::Symlink { path: "src/link", target: "file.txt" },
    ],
    pre_existing: &[],
    create_options: &["-L"],
    extract_options: &["-p"],
}

// L11: symlink_chain_depth4
Scenario {
    name: "L_symlink_chain_depth4",
    source_files: &[
        FileSpec::File { path: "src/final", contents: b"end", mtime: None,
                         mode: None, uid: None, gid: None },
        FileSpec::Symlink { path: "src/c", target: "final" },
        FileSpec::Symlink { path: "src/b", target: "c" },
        FileSpec::Symlink { path: "src/a", target: "b" },
    ],
    pre_existing: &[],
    create_options: &["-L"],
    extract_options: &[],
}
```

## Naming convention + fixture sharing

- 全 scenario 名に **`L_` prefix** で統一 (既存 framework の命名と整合: `unlink_basic`, `follow_symlink_P` 等のパターンと並列)
- L8 (strip_components), L9 (exclude), L14 (substitution) の **source_files は L2 と同一**で `create_options` のみ差。共通定数として抽出し、scenario 4 件で参照することで重複削減
- `pre_existing: &[]` は全 19 件で空 (extract destination は always empty で開始)

## Edge case safeguards

| 項目 | 値 / 方針 |
|---|---|
| symlink chain depth (L11) | **depth 4**。OS の `ELOOP` 上限 (Linux 40, macOS 32) より浅く、test 時間も増大させない |
| symlink loop test (L15/L16) | bats のみ + bats 内 `timeout 5` で外側強制終了。oracle に入れると hang リスク。exit 124 を expected に含める |
| dangling symlink (L4) | **実測済 (macOS libarchive 3.5.3)**: bsdtar `-cLf` dangling symlink は symlink として保持、exit 0、warning なし。PNA も `StoreAs::Symlink` で保持するため両者 parity 一致想定 |
| chain depth ≥ 4 で loop に陥らない構造 | fixture 作成時にレビュー、循環参照を排除 |

## Acceptance Criteria

| Stage | Done definition |
|---|---|
| Stage 1 (rebase) | `cargo xtask bsdtar-compat` が main 上で全既存 axes green、CI workflow 動作確認、conflict 0 件で PR 化 |
| Stage 2 (FsSnapshot 拡張) | mode/mtime/uid/gid の field 追加、既存 19 axes は `None` 既定で動作維持、新 metadata 比較が `--no-same-owner` / `-p` で発火し differential を検出可能 |
| Stage 3 (`-L` axis) | 14 oracle scenarios (Tier 1+2) 全 pass、bsdtar 実機が CI 上で利用可能 (`apt install libarchive-tools` / `brew install libarchive`)、Linux + macOS で network 検証成立 |
| Stage 4 (bats 補完) | 5 bats scenarios 全 pass、PNA 仕様確定 (loop / `-L -H` の意味論) を spec に追記 |

## Risks + Mitigations

| Risk | 対象 stage | Mitigation |
|---|---|---|
| ci/bsdtar-compat-labels の rebase で既存 axes が壊れる | 1 | rebase 前に branch fork、conflict resolution は私が実施しユーザー確認、main マージ前に local で全 axis green を確認 |
| Stage 2 の field 追加で既存 axes の expected が変わる (e.g., 0o644 が runtime で 0o755 になる platform 等) | 2 | `Option<u32>` の `None` は **比較から除外** する semantics で実装。既存 axes は `None` のまま → 既存 expected 不変 |
| bsdtar 実機の挙動が libarchive バージョンで異なる | 3 | CI で利用する libarchive バージョンを workflow に固定 (例: `apt install libarchive-tools=3.7.4-*`)、または oracle 出力を runtime 観測ベースに留め (= 同じ runner 上で bsdtar と PNA を両方走らせて差分のみ報告)、版数依存の expected を埋め込まない |
| symlink loop test で CI runner が hang | 4 | bats 内で `timeout 5` で外側強制終了、exit 124 を expected に含める |
| Windows reparse point の test 環境 | 4 | Windows runner で msys2 / cygwin の bsdtar が利用可能か事前確認、不可なら scenario skip + 別 design 化 |
| macOS の uid/gid が runtime の Mac OS X user に依存 | 3 (L13) | `runtime uid` を `getuid()` で取得、test 内で動的に expected と比較 (固定 uid を埋めない)。または `fakeroot` 利用 |
| dangling symlink (L4) の bsdtar 挙動を pre-design phase で特定できない | 3 | brainstorming 完了後 implementation 着手前に `bsdtar -cLf - -C src .` を実機で 1 回叩き、warn vs skip vs include の挙動を確認 → spec に明記 |

## Pre-implementation tasks

Stage 3 着手前に必ず実施:

1. **bsdtar 実機 dangling symlink 挙動の実測** (L4 prerequisite)
   - `bsdtar -cLf - -C src .` を Linux + macOS で実行
   - warn 出力 / exit code / archive 内容を記録
   - spec に observed behavior を inline で追記
2. **CI workflow に libarchive install 追記**
   - `.github/workflows/bsdtar-compat.yml` に `apt-get install libarchive-tools` (Ubuntu) / `brew install libarchive` (macOS) ステップ追加
   - libarchive バージョン版数固定の判断
3. **Stage 1 (rebase) の completion 確認**
   - `git rebase main` の conflict 解決
   - main 上で `cargo xtask bsdtar-compat` 実行 → 全既存 axis green

## Related specs

| 関連 spec | 場所 |
|---|---|
| Stage 2: FsSnapshot extension design | `docs/plans/2026-04-26-fs-snapshot-metadata-extension-design.md` (TBD、別 brainstorming) |
| Stage 4: -L bats supplement (本 spec の Tier 3 部分) | 本 spec に統合済 |
| xattr / ACL / fflags axis design | future, `--keep-*` 評価時 |
| 特殊ファイル (FIFO/device/socket) PNA scope-out 明文化 | future, separate design 推奨 |
| 既存 ci/bsdtar-compat-labels oracle framework | `docs/plans/2026-02-19-bsdtar-compat-oracle-design.md` |
| 既存 ci/bsdtar-compat-labels implementation plan | `docs/plans/2026-02-19-bsdtar-compat-oracle.md` |

## Implementation Outcome (Stage 3)

- **Date**: 2026-04-27 (UTC)
- **HEAD at run**: `a8ebb8ef`
- **Run result**: `317952 scenarios: 302921 passed, 14583 failed, 448 errors`
- **Run log**: `/tmp/bsdtar-compat-L.log` (local artifact)

### Fail breakdown by `<deref>_<entry-type>`

| Deref + entry | Failures | Notes |
|---|---|---|
| `L_SymDir` | 1207 | new variant; `-L` follow of symlink-to-dir → bsdtar/PNA divergence |
| `no_L_SymChain4` | 1130 | new variant; `-L` 無しの chain depth 4 symlink 保存挙動差 |
| `no_L_SymChain2` | 1118 | new variant; chain depth 2 同様 |
| `L_Dir` / `no_L_Dir` | 1103 / 1094 | existing axis; dereference 軸非依存の Dir 挙動差 |
| `L_Nested` / `no_L_Nested` | 992 / 992 | existing axis; same |
| `no_L_SymDir` | 895 | new variant; without `-L` の symlink-to-dir 保存挙動差 |
| `no_L_Sym` | 794 | existing axis; symlink 保存挙動差 |
| `L_SymDangling` / `no_L_SymDangling` | 694 / 692 | new variant; dangling 保持挙動差 (両方類似で deref 非影響) |
| `L_SymChain4` | 649 | new variant; `-L` chain depth 4 dereference |
| `L_Sym` | 626 | existing; `-L` symlink-to-file follow |
| `L_SymChain2` | 622 | new variant; `-L` chain depth 2 dereference |
| `L_HLink` / `no_L_HLink` | 597 / 583 | existing axis |
| `no_L_File` / `L_File` | 405 / 390 | existing baseline |

### Errors (448 total)

主要パターン: `no_L_File_over_Dir_keep_old_*: Permission denied (os error 13)` — extract destination が既存 Dir で `-k` (keep_old) 指定時、PNA が File を作ろうとして permission denied。**既存 PNA bug、`-L` axis 関係なし**。

### Categorization of `-L` related new variant failures

| Spec scenario | Pattern | Status |
|---|---|---|
| L1 (`L_baseline_no_dereference`, no `-L` Symlink) | `no_L_Sym` | 794 fails (既存 axis) |
| L2 (`L_symlink_to_file_dereferenced`, `-L` Symlink) | `L_Sym` | 626 fails |
| L3 (`L_symlink_to_dir_dereferenced`, `-L` SymDir) | `L_SymDir` | 1207 fails (新 variant、最多) |
| L4 (`L_dangling_symlink_with_L`, `-L` SymDangling) | `L_SymDangling` | 694 fails |
| L5 (`L_symlink_chain_2`, `-L` SymChain2) | `L_SymChain2` | 622 fails |
| L11 (`L_symlink_chain_depth4`, `-L` SymChain4) | `L_SymChain4` | 649 fails |

### Out-of-scope scenarios (Plan 想定だが Stage 2 確定の制約で実装せず)

| Scenario | Reason |
|---|---|
| L6 (cmdline-explicit path) | xtask は `-cf -C src .` 固定、cmdline-explicit path 軸を framework に持たない。Stage 4 bats supplement へ defer |
| L10 (target_permission_archived, `mode: Some(0o600)` 必要) | Stage 2 で `FileSpec` に mode 設定機能を持たない確定。fixture 側 metadata 設定不能。Stage 4 bats へ defer |
| L12 (target_mtime_archived, mtime 設定) | 同上、Stage 4 bats へ defer |
| L13 (target_uid_gid_archived, uid/gid 設定) | 同上、加えて chown 権限も必要。Stage 4 bats へ defer |
| L14 (substitution + dereference) | 既存 substitution axis (`s_lit`/`s_re`) に dereference 軸が掛かるため `L_Sym_*_s_*` パターンとして暗黙テスト中。専用 scenario 不要 |
| L7-L9 | 既存 axis (`-C dir .`, `--strip-components`, `--exclude`) に dereference 軸が掛かるため `L_*_strip1` / `L_*_excl` パターンとして暗黙テスト中。専用 scenario 不要 |

### Action items (Stage 3 範疇外、別 stage / issue で対応)

すべての fail axes を `docs/issues/2026-04-26-bsdtar-compat-post-rebase-fail-axis-tracker.md` の Stage 3 section に集計表で追記する (個別 axis 名の網羅は 14583 件で過大、entry-type ごとの集計に集約)。
