# SafeDir: openat ベース展開アーキテクチャ設計

## 動機

PNA の展開パイプラインは全ファイルシステム操作がパス（`PathBuf`）ベースで実装されている。パスサニタイズによる多層防御は正しく機能しているが、`ensure_directory_components` をはじめとする複数箇所で `symlink_metadata` → `create_dir` 間の TOCTOU（Time-of-Check to Time-of-Use）競合が構造的に存在する。

この脆弱性クラスは tar-rs（TARmageddon CVE-2025-62518）、zip-rs（CVE-2025-29787）で実際に悪用されている。PNA はパスサニタイズで十分に防御しているが、openat ベースのアーキテクチャに移行することで TOCTOU のクラス自体を根本的に排除する。

## 方針

- **セキュリティ最優先**: 実装コストよりも TOCTOU の根本排除を優先
- **cap-std + nix 補完**: cap-std でサンドボックスを確立し、cap-std が未対応の操作（chown, xattr, ACL）は `AsFd` 経由で nix に委譲
- **SafeDir 抽象レイヤー**: cap-std への直接依存を局所化し、将来の std openat 安定化時に移行を容易にする

## アーキテクチャ

### SafeDir の配置

```
cli/src/command/core/
├── safe_dir.rs          ← pub struct SafeDir, 共通型定義
├── safe_dir/
│   ├── openat_impl.rs   ← #[cfg(feature = "safe-dir")] cap-std ベース実装
│   └── path_impl.rs     ← #[cfg(not(feature = "safe-dir"))] パスベースフォールバック
```

- `libpna`（lib/）: 変更なし（ファイルシステム非依存の設計原則を維持）
- `pna`（pna/）: 変更なし
- CLI クレートのみに影響

### 依存関係

```toml
# cli/Cargo.toml
[features]
default = ["safe-dir"]
safe-dir = ["cap-std", "cap-fs-ext"]

[dependencies]
cap-std = { version = "4", optional = true }
cap-fs-ext = { version = "4", optional = true }

# 既存（変更なし）
[target.'cfg(unix)'.dependencies]
nix = { version = "0.31", features = ["user", "fs", "ioctl"] }
```

WASM ビルドでは `--exclude-features safe-dir` で除外。

## SafeDir API

```rust
pub struct SafeDir {
    inner: cap_std::fs::Dir,
    secure_symlinks: bool,
}

impl SafeDir {
    // --- 構築 ---
    pub fn open(path: &Path, secure_symlinks: bool) -> io::Result<Self>;
    pub fn open_dir(&self, path: &Path) -> io::Result<SafeDir>;
    pub fn try_clone(&self) -> io::Result<SafeDir>;

    // --- ファイル操作 ---
    pub fn create_file(&self, path: &Path, mode: u32, exclusive: bool) -> io::Result<File>;
    pub fn create_dir(&self, path: &Path, mode: u32) -> io::Result<()>;
    pub fn ensure_dir_all(&self, path: &Path, mode: u32) -> io::Result<()>;

    // --- リンク操作 ---
    pub fn symlink_contents(&self, target: &str, link: &Path) -> io::Result<()>;
    pub fn hard_link(&self, src: &Path, link: &Path) -> io::Result<()>;

    // --- メタデータ ---
    /// 戻り値は cfg によって切り替わる型エイリアス:
    /// - safe-dir feature 有効時: cap_std::fs::Metadata
    /// - safe-dir feature 無効時: std::fs::Metadata
    pub fn symlink_metadata(&self, path: &Path) -> io::Result<Metadata>;
    pub fn set_permissions(&self, path: &Path, mode: u32, no_follow: bool) -> io::Result<()>;
    /// atime/mtime が None の場合はそのフィールドを変更しない（SymbolicNow で上書きしない）
    pub fn set_times(&self, path: &Path, atime: Option<SystemTime>, mtime: Option<SystemTime>, no_follow: bool) -> io::Result<()>;

    // --- nix 補完操作（Unix のみ） ---
    /// no_follow=true でシンボリックリンク自体のメタデータを変更（lchown 相当）
    #[cfg(unix)]
    pub fn set_ownership(&self, path: &Path, uid: Option<u32>, gid: Option<u32>, no_follow: bool) -> io::Result<()>;
    #[cfg(unix)]
    pub fn set_xattr(&self, path: &Path, name: &str, value: &[u8]) -> io::Result<()>;
    #[cfg(unix)]
    pub fn get_xattr(&self, path: &Path, name: &str) -> io::Result<Option<Vec<u8>>>;

    // --- 削除・リネーム ---
    pub fn remove_file(&self, path: &Path) -> io::Result<()>;
    pub fn remove_dir(&self, path: &Path) -> io::Result<()>;
    pub fn remove_dir_all(&self, path: &Path) -> io::Result<()>;
    pub fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
}
```

### nix 補完操作の実装パターン

cap-std でパス解決（サンドボックス内に制限）→ nix の `*at` 系関数で fd 相対操作:

```rust
#[cfg(unix)]
pub fn set_ownership(
    &self,
    path: &Path,
    uid: Option<u32>,
    gid: Option<u32>,
    no_follow: bool,
) -> io::Result<()> {
    // 親ディレクトリを cap-std で開く（サンドボックス内に制限）
    let (parent_dir, file_name) = split_parent(path);
    let parent = self.inner.open_dir(parent_dir)?;
    let dirfd = parent.as_fd();
    let flag = if no_follow {
        AtFlags::AT_SYMLINK_NOFOLLOW
    } else {
        AtFlags::empty()
    };
    nix::unistd::fchownat(
        Some(dirfd),
        file_name,
        uid.map(Uid::from_raw),
        gid.map(Gid::from_raw),
        flag,
    )?;
    Ok(())
}
```

`fchownat(dirfd, name, AT_SYMLINK_NOFOLLOW)` により、シンボリックリンクを follow せずにリンク自体の所有者を変更できる。`fchown(fd)` ではなく `fchownat(dirfd, name)` を使うことで、dangling symlink のメタデータ設定にも対応。同じパターンを `set_permissions` や `set_times` のシンボリックリンク対応にも適用する。

## 展開パイプラインの変更

### データフロー

```
【現在】
OutputOption { out_dir: Option<PathBuf>, ... }
  → build_output_path(out_dir, name) → PathBuf
    → fs::File::create(abs_path)
    → ensure_directory_components(&abs_path, ...)

【変更後】
OutputOption { safe_dir: Option<SafeDir>, ... }
  → extract_entry(item, &name, &safe_dir, ...)
    → safe_dir.ensure_dir_all(parent, mode)
    → safe_dir.create_file(rel_path, mode, exclusive)
```

絶対パスの組み立て（`out_dir.join(name)`）が不要になる。`Path::join` による絶対パス置換リスクも同時に解消。

### 各 DataKind の変更

| DataKind | 現在 | 変更後 |
|---|---|---|
| File | `build_output_path` → `SafeWriter::new(abs_path)` or `file_create(abs_path)` | `safe_dir.ensure_dir_all(parent)` → `SafeWriter::new(&safe_dir, rel)` or `safe_dir.create_file(rel)` |
| Directory | `ensure_directory_components(abs_path)` | `safe_dir.ensure_dir_all(rel_path)` |
| SymbolicLink | `pna::fs::symlink(target, abs_path)` | `safe_dir.symlink_contents(target, rel_path)` |
| HardLink | `fs::hard_link(abs_src, abs_path)` | `safe_dir.hard_link(rel_src, rel_path)` |

### rayon 並列化

```rust
let safe_dir = SafeDir::open(out_dir, secure_symlinks)?;
rayon::scope_fifo(|scope| {
    for item in entries {
        let dir = safe_dir.try_clone()?;  // dup() で fd を複製
        scope.spawn_fifo(move |_| {
            extract_entry(item, &name, &dir, ...);
        });
    }
});
```

### SafeWriter の移行

`SafeWriter` は `SafeDir` への参照を持つ形にリファクタ。一時ファイル作成・アトミックリネームを `SafeDir` 経由で実行。

### build_output_path の廃止

サニタイズ済み相対パスをそのまま `SafeDir` のメソッドに渡すため、`build_output_path` は不要になる。

### pna compat bsdtar

bsdtar モードも `run_extract_archive_reader()` を共有するため、`OutputOption` に `SafeDir` を入れれば自動適用。`allow_unsafe_links` / `secure_symlinks` フラグは `SafeDir` のコンストラクタ引数で制御。

## フォールバック実装

`path_impl.rs` は現行コードのロジックを `SafeDir` API でラップ:

```rust
pub struct SafeDir {
    base_path: PathBuf,
    secure_symlinks: bool,
}
```

- WASM/Redox: 現行と同等のセキュリティレベル（パスベース + サニタイズ）
- それ以外: cap-std による openat ベースサンドボックス
- API は統一。呼び出し元に `#[cfg]` 分岐不要

## テスト戦略

### Layer 1: SafeDir ユニットテスト

攻撃パターンごとに検証:

- `../` エスケープが拒否されること
- シンボリックリンクコンポーネント経由のファイル作成が拒否されること
- 絶対パスが拒否されること
- Windows バックスラッシュ（`..\\`）が処理されること
- `ensure_dir_all` がシンボリックリンクコンポーネントを拒否すること
- ハードリンクがサンドボックス内に制限されること
- アトミックリネームが正しく動作すること

### Layer 2: 悪意あるアーカイブの統合テスト

実際の PNA アーカイブを作成・展開:

- Zip Slip パターン（symlink + file）
- `../` を含むエントリ名
- 絶対パスエントリ
- ハードリンク脱出
- bsdtar モードの `allow_unsafe_links` + `secure_symlinks` の組み合わせ

### Layer 3: 実装パリティテスト

`openat_impl` と `path_impl` の共通テスト関数で振る舞いの一致を検証。

### 既存テスト

既存の CLI 統合テスト（`cli/tests/cli/`）は展開結果のファイル内容を検証するため、`SafeDir` 導入後もそのまま動作する。全テスト通過が「振る舞いを壊していない」ことの証明。

## 移行フェーズ

### Phase 1: SafeDir 基盤構築

- `SafeDir` 構造体と `openat_impl` を実装
- `path_impl` フォールバックを実装
- `SafeDir` ユニットテスト

### Phase 2: 展開パイプラインの移行

- `OutputOption` に `SafeDir` を追加
- `extract_entry` の各分岐を `SafeDir` 呼び出しに変更
- `SafeWriter` を `SafeDir` ベースにリファクタ
- `ensure_directory_components` を `SafeDir::ensure_dir_all` に置換
- `build_output_path` を廃止

### Phase 3: メタデータ操作の移行

- chmod / timestamps を `SafeDir` 経由に変更
- chown を `SafeDir::set_ownership`（nix 経由）に変更
- xattr / ACL を `SafeDir` 経由の fd ベースに変更

### Phase 4: 検証とクリーンアップ

- 悪意あるアーカイブの統合テスト追加
- 全既存テストの通過確認
- 旧コードパス（`ensure_directory_components` 等）の削除
- ベンチマーク比較（展開速度への影響測定）

## リスクと対策

| リスク | 対策 |
|---|---|
| cap-std のフォールバック（旧カーネル）にも TOCTOU がある | 許容する。現行より改善されており、カーネルが対応すれば自動的に完全防護 |
| cap-std の API が要件を満たさないエッジケース | `AsFd` で生 fd を取得して nix に委譲 |
| 展開速度の低下（openat2 未対応環境） | Phase 4 でベンチマーク。問題があれば Dir キャッシュ等で最適化 |
| WASM フォールバックの挙動差異 | Layer 3 パリティテストで検出・明文化 |
| cap-std のメジャーバージョンアップ | SafeDir 抽象が変更を吸収 |
| exacl クレートが fd 非対応 | ACL のみパスベースで残す。リスク低（optional 機能） |

## 互換性保証

- CLI の振る舞い変更なし（コマンドライン引数、デフォルト値、エラーメッセージ維持）
- `libpna` / `pna` クレートは変更なし
- MSRV: cap-std は 1.70、PNA は 1.88 → 問題なし

## プラットフォーム別の防護レベル

| プラットフォーム | RESOLVE_BENEATH | TOCTOU 耐性 |
|---|---|---|
| Linux 5.6+ | `openat2(RESOLVE_BENEATH)` | 完全 |
| FreeBSD 14+ | `openat(O_RESOLVE_BENEATH)` | 完全 |
| macOS / 旧 Linux / 他 Unix | コンポーネント逐次走査 | 並行攻撃なしでは安全 |
| Windows | `NtCreateFile` + `RootDirectory` | コンポーネント逐次走査相当 |
| WASM / Redox | パスベースフォールバック | 現行同等 |
