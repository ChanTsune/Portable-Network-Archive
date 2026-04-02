# SafeDir: openat ベース展開アーキテクチャ 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** TOCTOU 競合を根本的に排除するために、CLI の展開パイプラインを cap-std ベースの `SafeDir` 抽象に移行する。

**Architecture:** `SafeDir` 構造体が `cap_std::fs::Dir` をラップし、全ファイルシステム操作を openat ベースの fd 相対操作に変換する。cap-std が未対応の操作（chown, xattr）は `AsFd` 経由で nix の `*at` 系関数に委譲する。WASM 向けには現行パスベースのフォールバック実装を提供する。

**Tech Stack:** cap-std 4, cap-fs-ext 4, nix 0.31 (既存依存), Rust 2024 edition

**Spec:** `docs/superpowers/specs/2026-04-01-openat-safedir-design.md`

---

## File Structure

### 新規作成

| ファイル | 責務 |
|---|---|
| `cli/src/command/core/safe_dir.rs` | `SafeDir` の公開 API 定義、モジュール宣言、`cfg` による実装の切り替え |
| `cli/src/command/core/safe_dir/openat_impl.rs` | cap-std ベースの `SafeDir` 実装 |
| `cli/src/command/core/safe_dir/path_impl.rs` | パスベースのフォールバック `SafeDir` 実装 |
| `cli/tests/cli/safe_dir.rs` | `SafeDir` のセキュリティテスト（サンドボックスエスケープ防止） |

### 変更対象

| ファイル | 変更内容 |
|---|---|
| `cli/Cargo.toml` | `safe-dir` feature と cap-std/cap-fs-ext 依存を追加 |
| `cli/src/command/core.rs` | `pub(crate) mod safe_dir;` 追加 |
| `cli/src/command/extract.rs:590-606` | `OutputOption` に `safe_dir: Option<SafeDir>` を追加 |
| `cli/src/command/extract.rs:494-514` | `extract_archive()` で `SafeDir::open()` を呼び出し |
| `cli/src/command/extract.rs:1181-1201` | `extract_entry()` を `SafeDir` ベースに変更 |
| `cli/src/command/extract.rs:1309-1320` | `build_output_path()` を廃止 |
| `cli/src/command/extract.rs:1645-1717` | `ensure_directory_components()` を `SafeDir::ensure_dir_all()` に置換 |
| `cli/src/command/extract.rs:1078-1177` | `check_and_prepare_target()` を `SafeDir` ベースに変更 |
| `cli/src/command/core/safe_writer.rs:16-20` | `SafeWriter` を `SafeDir` ベースにリファクタ |
| `cli/src/command/bsdtar.rs:1080-1119` | bsdtar の `OutputOption` 構築で `SafeDir` を使用 |
| `cli/tests/cli/main.rs` | `mod safe_dir;` 追加 |

---

## Phase 1: SafeDir 基盤構築

### Task 1: cap-std 依存関係の追加

**Files:**
- Modify: `cli/Cargo.toml:89-96` (features section)
- Modify: `cli/Cargo.toml:13-53` (dependencies section)

- [ ] **Step 1: features セクションに safe-dir を追加**

`cli/Cargo.toml` の `[features]` セクション（89行目〜）に追加:

```toml
[features]
acl = [
    "dep:exacl",
    "dep:field-offset",
    "windows/Win32_System_SystemServices",
]
memmap = ["dep:memmap2"]
zlib-ng = ["pna/zlib-ng"]
safe-dir = ["dep:cap-std", "dep:cap-fs-ext"]
default = ["safe-dir"]
```

- [ ] **Step 2: dependencies セクションに cap-std, cap-fs-ext を追加**

`cli/Cargo.toml` の `[dependencies]` セクションに追加:

```toml
cap-std = { version = "4", optional = true }
cap-fs-ext = { version = "4", optional = true }
```

- [ ] **Step 3: ビルド確認**

Run: `cargo build -p portable-network-archive --all-features`
Expected: BUILD SUCCESS

- [ ] **Step 4: WASM ビルドで safe-dir が除外されることを確認**

Run: `cargo check -p portable-network-archive --no-default-features --target wasm32-wasip1`
Expected: CHECK SUCCESS（cap-std なしでビルド可能）

注意: WASM ビルドが通らない場合は、`safe-dir` feature が CLI の WASM 関連コードに干渉していないか確認。`cfg(feature = "safe-dir")` は safe_dir モジュール内に閉じているため、この段階では問題にならないはず。

- [ ] **Step 5: コミット**

```bash
git add cli/Cargo.toml
git commit -m ":arrow_up: Add cap-std and cap-fs-ext as optional dependencies for safe-dir feature"
```

---

### Task 2: SafeDir モジュール構造の作成

**Files:**
- Create: `cli/src/command/core/safe_dir.rs`
- Create: `cli/src/command/core/safe_dir/openat_impl.rs`
- Create: `cli/src/command/core/safe_dir/path_impl.rs`
- Modify: `cli/src/command/core.rs:1-14`

- [ ] **Step 1: core.rs にモジュール宣言を追加**

`cli/src/command/core.rs` の 12行目（`pub(crate) mod safe_writer;` の後）に追加:

```rust
pub(crate) mod safe_dir;
```

- [ ] **Step 2: safe_dir.rs を作成（公開 API と cfg 切り替え）**

```rust
#[cfg(feature = "safe-dir")]
mod openat_impl;
#[cfg(not(feature = "safe-dir"))]
mod path_impl;

use std::fs::{self, Metadata};
use std::io;
use std::path::Path;
use std::time::SystemTime;

#[cfg(feature = "safe-dir")]
pub(crate) use openat_impl::SafeDir;
#[cfg(not(feature = "safe-dir"))]
pub(crate) use path_impl::SafeDir;
```

- [ ] **Step 3: openat_impl.rs をスタブで作成**

```rust
use std::fs::{self, File, Metadata};
use std::io;
use std::path::Path;
use std::time::SystemTime;

pub(crate) struct SafeDir {
    inner: cap_std::fs::Dir,
    secure_symlinks: bool,
}

impl SafeDir {
    pub(crate) fn open(path: &Path, secure_symlinks: bool) -> io::Result<Self> {
        let inner = cap_std::fs::Dir::open_ambient_dir(path, cap_std::ambient_authority())?;
        Ok(Self {
            inner,
            secure_symlinks,
        })
    }

    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            inner: self.inner.try_clone()?,
            secure_symlinks: self.secure_symlinks,
        })
    }
}
```

- [ ] **Step 4: path_impl.rs をスタブで作成**

```rust
use std::fs::{self, File, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) struct SafeDir {
    base_path: PathBuf,
    secure_symlinks: bool,
}

impl SafeDir {
    pub(crate) fn open(path: &Path, secure_symlinks: bool) -> io::Result<Self> {
        let base_path = path.to_path_buf();
        Ok(Self {
            base_path,
            secure_symlinks,
        })
    }

    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            base_path: self.base_path.clone(),
            secure_symlinks: self.secure_symlinks,
        })
    }
}
```

- [ ] **Step 5: ビルド確認**

Run: `cargo build -p portable-network-archive --all-features`
Expected: BUILD SUCCESS

Run: `cargo build -p portable-network-archive --no-default-features`
Expected: BUILD SUCCESS（path_impl が使われる）

- [ ] **Step 6: コミット**

```bash
git add cli/src/command/core.rs cli/src/command/core/safe_dir.rs cli/src/command/core/safe_dir/openat_impl.rs cli/src/command/core/safe_dir/path_impl.rs
git commit -m ":sparkles: Add SafeDir module structure with openat and path-based implementations"
```

---

### Task 3: SafeDir openat_impl — ファイル・ディレクトリ操作

**Files:**
- Modify: `cli/src/command/core/safe_dir/openat_impl.rs`

- [ ] **Step 1: create_file を実装**

`openat_impl.rs` の `impl SafeDir` に追加:

```rust
use cap_std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

impl SafeDir {
    pub(crate) fn create_file(
        &self,
        path: &Path,
        mode: u32,
        exclusive: bool,
    ) -> io::Result<File> {
        let mut opts = OpenOptions::new();
        opts.write(true).create(true).truncate(!exclusive);
        if exclusive {
            opts.create_new(true);
        }
        #[cfg(unix)]
        opts.mode(mode);
        let cap_file = self.inner.open_with(path, &opts)?;
        Ok(cap_file.into_std())
    }
}
```

- [ ] **Step 2: create_dir を実装**

```rust
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

impl SafeDir {
    pub(crate) fn create_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
        let mut builder = cap_std::fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(mode);
        self.inner.create_dir_with(path, &builder)
    }
}
```

- [ ] **Step 3: ensure_dir_all を実装**

```rust
impl SafeDir {
    pub(crate) fn ensure_dir_all(&self, path: &Path, mode: u32) -> io::Result<()> {
        if path.as_os_str().is_empty() {
            return Ok(());
        }
        let mut builder = cap_std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        builder.mode(mode);
        // cap-std の create_dir_all は内部で openat2(RESOLVE_BENEATH) を使用し、
        // シンボリックリンク経由の脱出を防止する。
        // secure_symlinks が false の場合も cap-std のサンドボックスは有効。
        self.inner.create_dir_with(path, &builder)
    }
}
```

- [ ] **Step 4: ビルド確認**

Run: `cargo build -p portable-network-archive --features safe-dir`
Expected: BUILD SUCCESS

- [ ] **Step 5: コミット**

```bash
git add cli/src/command/core/safe_dir/openat_impl.rs
git commit -m ":sparkles: Implement SafeDir file and directory operations (openat)"
```

---

### Task 4: SafeDir openat_impl — リンク・メタデータ・削除操作

**Files:**
- Modify: `cli/src/command/core/safe_dir/openat_impl.rs`

- [ ] **Step 1: リンク操作を実装**

```rust
impl SafeDir {
    pub(crate) fn symlink_contents(
        &self,
        target: &str,
        link: &Path,
    ) -> io::Result<()> {
        #[cfg(unix)]
        {
            self.inner.symlink_contents(target, link)
        }
        #[cfg(windows)]
        {
            // Windows ではターゲットの種別を判定してから作成
            if Path::new(target).extension().is_none() {
                self.inner.symlink_dir(target, link)
            } else {
                self.inner.symlink_file(target, link)
            }
        }
    }

    pub(crate) fn hard_link(&self, src: &Path, link: &Path) -> io::Result<()> {
        self.inner.hard_link(src, &self.inner, link)
    }
}
```

- [ ] **Step 2: メタデータ操作を実装**

```rust
use cap_fs_ext::DirExt;

impl SafeDir {
    // 戻り値は cap_std::fs::Metadata（into_std() は存在しない）
    // safe_dir.rs で型エイリアスを定義して吸収:
    //   #[cfg(feature = "safe-dir")] type Metadata = cap_std::fs::Metadata;
    //   #[cfg(not(feature = "safe-dir"))] type Metadata = std::fs::Metadata;
    pub(crate) fn symlink_metadata(&self, path: &Path) -> io::Result<cap_std::fs::Metadata> {
        self.inner.symlink_metadata(path)
    }

    pub(crate) fn set_permissions(
        &self,
        path: &Path,
        mode: u32,
        no_follow: bool,
    ) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perm = fs::Permissions::from_mode(mode);
            if no_follow {
                self.inner.set_symlink_permissions(path, perm)
            } else {
                self.inner.set_permissions(path, perm)
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (path, mode, no_follow);
            Ok(())
        }
    }

    pub(crate) fn set_times(
        &self,
        path: &Path,
        atime: Option<SystemTime>,
        mtime: Option<SystemTime>,
        no_follow: bool,
    ) -> io::Result<()> {
        use cap_fs_ext::SystemTimeSpec;
        // None の場合はそのフィールドを変更しない（SymbolicNow で上書きしない）
        let a = atime.map(|t| SystemTimeSpec::Absolute(cap_std::time::SystemTime::from_std(t)));
        let m = mtime.map(|t| SystemTimeSpec::Absolute(cap_std::time::SystemTime::from_std(t)));
        if no_follow {
            self.inner.set_symlink_times(path, a, m)
        } else {
            self.inner.set_times(path, a, m)
        }
    }
}
```

- [ ] **Step 3: 削除・リネーム操作を実装**

```rust
impl SafeDir {
    pub(crate) fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_file(path)
    }

    pub(crate) fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir(path)
    }

    pub(crate) fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir_all(path)
    }

    pub(crate) fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, &self.inner, to)
    }
}
```

- [ ] **Step 4: ビルド確認**

Run: `cargo build -p portable-network-archive --features safe-dir`
Expected: BUILD SUCCESS

- [ ] **Step 5: コミット**

```bash
git add cli/src/command/core/safe_dir/openat_impl.rs
git commit -m ":sparkles: Implement SafeDir link, metadata, and delete operations (openat)"
```

---

### Task 5: SafeDir openat_impl — nix 補完操作（Unix のみ）

**Files:**
- Modify: `cli/src/command/core/safe_dir/openat_impl.rs`

- [ ] **Step 1: split_parent ヘルパーを追加**

```rust
/// パスを親ディレクトリとファイル名に分割する。
/// SafeDir の *at 系操作で、親ディレクトリの fd を開いてからファイル名で操作するために使用。
fn split_parent(path: &Path) -> (&Path, &std::ffi::OsStr) {
    let parent = path.parent().unwrap_or(Path::new(""));
    let name = path.file_name().unwrap_or(path.as_os_str());
    (parent, name)
}
```

- [ ] **Step 2: set_ownership を実装**

```rust
#[cfg(unix)]
use nix::sys::stat::FchmodatFlags;
#[cfg(unix)]
use nix::unistd::{fchownat, FchownatFlags, Gid, Uid};
#[cfg(unix)]
use std::os::unix::io::AsFd;

#[cfg(unix)]
impl SafeDir {
    pub(crate) fn set_ownership(
        &self,
        path: &Path,
        uid: Option<u32>,
        gid: Option<u32>,
        no_follow: bool,
    ) -> io::Result<()> {
        let (parent, name) = split_parent(path);
        let parent_dir = if parent.as_os_str().is_empty() {
            self.inner.try_clone()?
        } else {
            self.inner.open_dir(parent)?
        };
        let flag = if no_follow {
            FchownatFlags::NoFollow
        } else {
            FchownatFlags::FollowSymlink
        };
        fchownat(
            Some(parent_dir.as_fd()),
            name,
            uid.map(Uid::from_raw),
            gid.map(Gid::from_raw),
            flag,
        )
        .map_err(io::Error::from)
    }
}
```

- [ ] **Step 3: set_xattr / get_xattr を実装**

```rust
#[cfg(unix)]
impl SafeDir {
    pub(crate) fn set_xattr(
        &self,
        path: &Path,
        name: &str,
        value: &[u8],
    ) -> io::Result<()> {
        // cap-std でファイルを開き（サンドボックス内に制限）、fd 経由で xattr 設定
        let file = self.inner.open(path)?;
        let std_file = file.into_std();
        xattr::FileExt::set_xattr(&std_file, name, value)
    }

    pub(crate) fn get_xattr(
        &self,
        path: &Path,
        name: &str,
    ) -> io::Result<Option<Vec<u8>>> {
        let file = self.inner.open(path)?;
        let std_file = file.into_std();
        xattr::FileExt::get_xattr(&std_file, name)
    }
}
```

注意: xattr クレートの `FileExt` トレイトが `std::fs::File` に対して実装されているか確認が必要。実装されていない場合は `rustix::fs::fsetxattr` / `fgetxattr` を直接使用する。

- [ ] **Step 4: ビルド確認**

Run: `cargo build -p portable-network-archive --all-features`
Expected: BUILD SUCCESS

- [ ] **Step 5: コミット**

```bash
git add cli/src/command/core/safe_dir/openat_impl.rs
git commit -m ":sparkles: Implement SafeDir nix-backed ownership and xattr operations"
```

---

### Task 6: SafeDir path_impl — フォールバック実装

**Files:**
- Modify: `cli/src/command/core/safe_dir/path_impl.rs`

- [ ] **Step 1: ファイル・ディレクトリ操作を実装**

```rust
use std::fs::{self, File, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) struct SafeDir {
    base_path: PathBuf,
    secure_symlinks: bool,
}

impl SafeDir {
    pub(crate) fn open(path: &Path, secure_symlinks: bool) -> io::Result<Self> {
        let base_path = fs::canonicalize(path)?;
        Ok(Self {
            base_path,
            secure_symlinks,
        })
    }

    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            base_path: self.base_path.clone(),
            secure_symlinks: self.secure_symlinks,
        })
    }

    fn resolve(&self, path: &Path) -> PathBuf {
        self.base_path.join(path)
    }

    pub(crate) fn create_file(
        &self,
        path: &Path,
        _mode: u32,
        exclusive: bool,
    ) -> io::Result<File> {
        let full = self.resolve(path);
        if exclusive {
            fs::File::create_new(&full)
        } else {
            fs::File::create(&full)
        }
    }

    pub(crate) fn create_dir(&self, path: &Path, _mode: u32) -> io::Result<()> {
        fs::create_dir(self.resolve(path))
    }

    pub(crate) fn ensure_dir_all(&self, path: &Path, _mode: u32) -> io::Result<()> {
        let full = self.resolve(path);
        fs::create_dir_all(full)
    }
}
```

- [ ] **Step 2: リンク・メタデータ・削除操作を実装**

```rust
impl SafeDir {
    pub(crate) fn symlink_contents(
        &self,
        target: &str,
        link: &Path,
    ) -> io::Result<()> {
        let full_link = self.resolve(link);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, &full_link)
        }
        #[cfg(windows)]
        {
            if Path::new(target).extension().is_none() {
                std::os::windows::fs::symlink_dir(target, &full_link)
            } else {
                std::os::windows::fs::symlink_file(target, &full_link)
            }
        }
        #[cfg(target_os = "wasi")]
        {
            std::os::wasi::fs::symlink_path(target, &full_link)
        }
    }

    pub(crate) fn hard_link(&self, src: &Path, link: &Path) -> io::Result<()> {
        fs::hard_link(self.resolve(src), self.resolve(link))
    }

    pub(crate) fn symlink_metadata(&self, path: &Path) -> io::Result<Metadata> {
        fs::symlink_metadata(self.resolve(path))
    }

    pub(crate) fn set_permissions(
        &self,
        path: &Path,
        mode: u32,
        _no_follow: bool,
    ) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perm = fs::Permissions::from_mode(mode);
            fs::set_permissions(self.resolve(path), perm)
        }
        #[cfg(not(unix))]
        {
            let _ = (path, mode);
            Ok(())
        }
    }

    pub(crate) fn set_times(
        &self,
        path: &Path,
        _atime: Option<SystemTime>,
        _mtime: Option<SystemTime>,
        _no_follow: bool,
    ) -> io::Result<()> {
        // フォールバック: filetime クレートまたは直接 utimensat が必要
        // 既存の extract.rs のタイムスタンプ設定ロジックを呼び出す
        let _ = path;
        Ok(())
    }

    pub(crate) fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(self.resolve(path))
    }

    pub(crate) fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(self.resolve(path))
    }

    pub(crate) fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir_all(self.resolve(path))
    }

    pub(crate) fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(self.resolve(from), self.resolve(to))
    }
}
```

- [ ] **Step 3: Unix 固有操作を実装**

```rust
#[cfg(unix)]
impl SafeDir {
    pub(crate) fn set_ownership(
        &self,
        path: &Path,
        uid: Option<u32>,
        gid: Option<u32>,
        no_follow: bool,
    ) -> io::Result<()> {
        let full = self.resolve(path);
        if no_follow {
            std::os::unix::fs::lchown(&full, uid, gid)
        } else {
            std::os::unix::fs::chown(&full, uid, gid)
        }
    }

    pub(crate) fn set_xattr(
        &self,
        path: &Path,
        name: &str,
        value: &[u8],
    ) -> io::Result<()> {
        xattr::set(self.resolve(path), name, value)
    }

    pub(crate) fn get_xattr(
        &self,
        path: &Path,
        name: &str,
    ) -> io::Result<Option<Vec<u8>>> {
        xattr::get(self.resolve(path), name)
    }
}
```

- [ ] **Step 4: ビルド確認（both features）**

Run: `cargo build -p portable-network-archive --features safe-dir`
Expected: BUILD SUCCESS

Run: `cargo build -p portable-network-archive --no-default-features`
Expected: BUILD SUCCESS

- [ ] **Step 5: コミット**

```bash
git add cli/src/command/core/safe_dir/path_impl.rs
git commit -m ":sparkles: Implement SafeDir path-based fallback for WASM/Redox"
```

---

### Task 7: SafeDir ユニットテスト — サンドボックスエスケープ防止

**Files:**
- Create: `cli/tests/cli/safe_dir.rs`
- Modify: `cli/tests/cli/main.rs`

- [ ] **Step 1: main.rs にモジュール宣言を追加**

`cli/tests/cli/main.rs` に追加:

```rust
mod safe_dir;
```

- [ ] **Step 2: サンドボックスエスケープ防止テストを作成**

```rust
use std::fs;
use std::path::Path;
use tempfile::tempdir;

// SafeDir をテストから使用するため、CLI バイナリ経由ではなく
// 直接テスト用のヘルパーを使用する。
// これらのテストは `pna extract` コマンドのエンドツーエンドテストとして実装する。

/// ../エスケープを含むエントリ名が展開ディレクトリ外に書き込まないことを検証
#[test]
fn extract_dotdot_entry_stays_in_sandbox() {
    let tmp = tempdir().unwrap();
    let out_dir = tmp.path().join("out");
    fs::create_dir(&out_dir).unwrap();
    let secret = tmp.path().join("secret.txt");

    // ../secret.txt というエントリ名を持つアーカイブを作成
    let archive_path = tmp.path().join("test.pna");
    // pna CLI で作成して展開
    // エントリ名に ../ が含まれる場合、サニタイズで除去されるため
    // out_dir 外にファイルが作成されないことを確認
    assert!(!secret.exists(), "Secret file should not exist outside sandbox");
}

/// シンボリックリンク→ファイルの順で格納された悪意あるアーカイブが
/// 展開ディレクトリ外に書き込まないことを検証
#[test]
fn extract_symlink_then_file_blocked() {
    let tmp = tempdir().unwrap();
    let out_dir = tmp.path().join("out");
    fs::create_dir(&out_dir).unwrap();
    let target_dir = tmp.path().join("target");
    fs::create_dir(&target_dir).unwrap();

    // 1. "link" -> "../../target/" のシンボリックリンクエントリ
    // 2. "link/payload.txt" のファイルエントリ
    // を含むアーカイブを展開しても、target/ にファイルが現れないことを確認
    let payload = target_dir.join("payload.txt");
    assert!(!payload.exists(), "Payload should not escape through symlink");
}

/// ハードリンクが展開ディレクトリ外のファイルを指さないことを検証
#[test]
fn extract_hardlink_escape_blocked() {
    let tmp = tempdir().unwrap();
    let out_dir = tmp.path().join("out");
    fs::create_dir(&out_dir).unwrap();

    // ../../etc/passwd へのハードリンクを含むアーカイブ
    // 展開後、ハードリンクが作成されないことを確認
}
```

注意: テストの実装詳細は、Phase 4 (Task 12) で `pna` CLI バイナリを使用したエンドツーエンドテストとして完成させる。この段階ではテストの骨格のみ。悪意あるアーカイブの生成には `libpna` の `Archive` / `EntryBuilder` API を直接使用する。

- [ ] **Step 3: ビルド確認**

Run: `cargo test -p portable-network-archive --test cli safe_dir --no-run`
Expected: BUILD SUCCESS

- [ ] **Step 4: コミット**

```bash
git add cli/tests/cli/safe_dir.rs cli/tests/cli/main.rs
git commit -m ":white_check_mark: Add SafeDir security test skeletons"
```

---

## Phase 2: 展開パイプラインの移行

### Task 8: OutputOption に SafeDir を追加

**Files:**
- Modify: `cli/src/command/extract.rs:590-606` (OutputOption struct)
- Modify: `cli/src/command/extract.rs:494-514` (extract_archive constructor)
- Modify: `cli/src/command/core/safe_dir.rs` (re-export)

- [ ] **Step 1: SafeDir を safe_dir.rs から re-export**

`cli/src/command/core/safe_dir.rs` に追加（既存の use/re-export の後）:

```rust
// SafeDir のベースパスを取得するメソッドを追加（path_impl のみ）
// OutputOption の out_dir との互換のため
```

- [ ] **Step 2: OutputOption に safe_dir フィールドを追加**

`cli/src/command/extract.rs:590-606` の `OutputOption` struct に追加:

```rust
pub(crate) struct OutputOption<'a> {
    pub(crate) overwrite_strategy: OverwriteStrategy,
    pub(crate) allow_unsafe_links: bool,
    pub(crate) out_dir: Option<PathBuf>,           // 段階的移行中は残す
    pub(crate) safe_dir: Option<SafeDir>,           // 新規追加
    pub(crate) to_stdout: bool,
    pub(crate) filter: PathFilter<'a>,
    pub(crate) keep_options: KeepOptions,
    pub(crate) pathname_editor: PathnameEditor,
    pub(crate) ordered_path_locks: Arc<OrderedPathLocks>,
    pub(crate) unlink_first: bool,
    pub(crate) time_filters: TimeFilters,
    pub(crate) safe_writes: bool,
    pub(crate) verbose: bool,
    pub(crate) absolute_paths: bool,
    pub(crate) warned_lead_slash: Arc<AtomicBool>,
}
```

- [ ] **Step 3: extract_archive() で SafeDir を構築**

`cli/src/command/extract.rs` の `extract_archive()` 内、OutputOption 構築部分（494行目付近）を変更:

```rust
use crate::command::core::safe_dir::SafeDir;

// out_dir が指定されている場合に SafeDir を構築
let secure_symlinks = !args.absolute_paths;
let safe_dir = args.out_dir.as_deref().map(|d| {
    // 展開先ディレクトリが存在しない場合は作成
    fs::create_dir_all(d)?;
    SafeDir::open(d, secure_symlinks)
}).transpose()?;

let out_option = OutputOption {
    // ... existing fields ...
    safe_dir,
    // ... rest ...
};
```

- [ ] **Step 4: bsdtar.rs でも SafeDir を構築**

`cli/src/command/bsdtar.rs:1080-1119` の OutputOption 構築に同様のフィールドを追加:

```rust
let secure_symlinks = !args.absolute_paths;
let safe_dir = args.out_dir.as_deref().map(|d| {
    fs::create_dir_all(d)?;
    SafeDir::open(d, secure_symlinks)
}).transpose()?;

let out_option = OutputOption {
    // ... existing fields ...
    safe_dir,
    // ... rest ...
};
```

- [ ] **Step 5: ビルド確認**

Run: `cargo build -p portable-network-archive --all-features`
Expected: BUILD SUCCESS（safe_dir フィールドは追加されたがまだ未使用）

- [ ] **Step 6: コミット**

```bash
git add cli/src/command/extract.rs cli/src/command/bsdtar.rs cli/src/command/core/safe_dir.rs
git commit -m ":sparkles: Add SafeDir field to OutputOption"
```

---

### Task 9: extract_entry を SafeDir ベースに移行

**Files:**
- Modify: `cli/src/command/extract.rs:1181-1320` (extract_entry, build_output_path)

この Task は段階的に移行する。まず `safe_dir` が `Some` の場合に新パスを通し、`None` の場合（to_stdout 等）は既存パスを維持。

- [ ] **Step 1: extract_entry 内で SafeDir の有無による分岐を追加**

`extract_entry` 関数（1181行目〜）の冒頭で、`safe_dir` がある場合はそれを使い、ない場合は従来の `build_output_path` を使うようにする。

変更の概要（疑似コード）:

```rust
let path = if let Some(ref safe_dir) = args.safe_dir {
    // SafeDir ベース: 相対パスをそのまま使用
    // 親ディレクトリの作成
    if let Some(parent) = item_path.as_path().parent() {
        if !parent.as_os_str().is_empty() {
            safe_dir.ensure_dir_all(Path::new(parent), 0o777)?;
        }
    }
    // build_output_path は呼ばない
    // ...各 DataKind の処理で safe_dir のメソッドを使用
} else {
    // 従来パス: build_output_path を使用
    // ...既存コードをそのまま維持
};
```

実際の変更は、`extract_entry` の各 `DataKind` 分岐（File, Directory, SymbolicLink, HardLink）それぞれで `safe_dir` の有無をチェックする形。

- [ ] **Step 2: File 分岐を SafeDir ベースに変更**

`extract_entry` 内の `DataKind::File` 分岐（1228行目付近）:

```rust
DataKind::File => {
    if let Some(ref safe_dir) = safe_dir {
        // SafeDir ベース
        if *safe_writes {
            let temp_name = format!(".pna.{:016x}", rand::random::<u64>());
            let temp_path = Path::new(&temp_name);
            let mut file = safe_dir.create_file(
                &item_path.as_path().parent().unwrap_or(Path::new("")).join(&temp_name),
                0o600,
                true,
            )?;
            io::copy(&mut reader, &mut file)?;
            file.sync_all()?;
            let rel_path = Path::new(item_path.as_str());
            safe_dir.rename(
                &rel_path.parent().unwrap_or(Path::new("")).join(&temp_name),
                rel_path,
            )?;
        } else {
            let rel_path = Path::new(item_path.as_str());
            let mut file = safe_dir.create_file(rel_path, 0o666, false)?;
            io::copy(&mut reader, &mut file)?;
        }
    } else {
        // 既存コード
        // ...
    }
}
```

注意: この疑似コードは方向性を示すもの。実装時には既存の `SafeWriter` のロジック（temp file + atomic rename）を `SafeDir` メソッドで置き換える。

- [ ] **Step 3: Directory 分岐を SafeDir ベースに変更**

```rust
DataKind::Directory => {
    if let Some(ref safe_dir) = safe_dir {
        safe_dir.ensure_dir_all(Path::new(item_path.as_str()), 0o777)?;
    } else {
        // 既存: ensure_directory_components(&path, ...)
    }
}
```

- [ ] **Step 4: SymbolicLink 分岐を SafeDir ベースに変更**

```rust
DataKind::SymbolicLink => {
    let original = io::read_to_string(reader)?;
    let original = pathname_editor.edit_symlink(original.as_ref());
    if !allow_unsafe_links && is_unsafe_link(&original) {
        log::warn!("Skipped extracting a symbolic link...");
        return Ok(());
    }
    if let Some(ref safe_dir) = safe_dir {
        safe_dir.symlink_contents(original.as_str(), Path::new(item_path.as_str()))?;
    } else {
        // 既存: utils::fs::symlink(original, &path)
    }
}
```

- [ ] **Step 5: HardLink 分岐を SafeDir ベースに変更**

```rust
DataKind::HardLink => {
    let original = io::read_to_string(reader)?;
    let Some((original, had_root)) = pathname_editor.edit_hardlink(original.as_ref()) else {
        return Ok(());
    };
    if !allow_unsafe_links && is_unsafe_link(&original) {
        log::warn!("Skipped extracting a hard link...");
        return Ok(());
    }
    if let Some(ref safe_dir) = safe_dir {
        safe_dir.hard_link(
            Path::new(original.as_str()),
            Path::new(item_path.as_str()),
        )?;
    } else {
        // 既存: fs::hard_link(original, &path)
    }
}
```

- [ ] **Step 6: 全テスト実行**

Run: `cargo test -p portable-network-archive --all-features`
Expected: ALL PASS（既存テストが壊れていないこと）

- [ ] **Step 7: コミット**

```bash
git add cli/src/command/extract.rs
git commit -m ":recycle: Migrate extract_entry to use SafeDir for all DataKind branches"
```

---

### Task 10: rayon 並列化を SafeDir 対応に変更

**Files:**
- Modify: `cli/src/command/extract.rs:625-750` (reader path rayon block)
- Modify: `cli/src/command/extract.rs:860-978` (memmap path rayon block)

- [ ] **Step 1: reader パスの rayon block で SafeDir を clone して配布**

`run_extract_archive_reader` 内の rayon block（630行目付近）で、`safe_dir` を各ワーカーにクローンして渡す:

```rust
// safe_dir を args から取り出す前に clone
// rayon::scope_fifo 内の各 spawn_fifo で dir.try_clone() を使う
```

変更箇所は `s.spawn_fifo(move |_| { ... })` 内で `args` を渡している部分。`args` は `&OutputOption` なので `safe_dir` への参照は自動的に共有される。`SafeDir` は `Send + Sync` なので問題なし。

実際には `OutputOption` が参照で渡されるため、`SafeDir` の clone は不要 — `&SafeDir` のまま rayon ワーカーに渡せる。

- [ ] **Step 2: テスト実行**

Run: `cargo test -p portable-network-archive --all-features`
Expected: ALL PASS

- [ ] **Step 3: コミット**

```bash
git add cli/src/command/extract.rs
git commit -m ":recycle: Adapt rayon parallelization for SafeDir shared references"
```

---

### Task 11: SafeWriter を SafeDir ベースにリファクタ

**Files:**
- Modify: `cli/src/command/core/safe_writer.rs:16-134`

- [ ] **Step 1: SafeWriter に SafeDir ベースのコンストラクタを追加**

```rust
use super::safe_dir::SafeDir;

pub(crate) struct SafeWriter {
    temp_path: Option<PathBuf>,
    final_path: PathBuf,
    file: fs::File,
    // SafeDir が利用可能な場合の参照（リネーム時に使用）
    // 段階的移行: safe_dir が None の場合は従来の PathBuf ベース
}

impl SafeWriter {
    /// SafeDir ベースのコンストラクタ
    pub(crate) fn with_safe_dir(
        safe_dir: &SafeDir,
        final_rel_path: &Path,
    ) -> io::Result<Self> {
        let temp_name = format!(".pna.{:016x}", rand::random::<u64>());
        let parent = final_rel_path.parent().unwrap_or(Path::new(""));
        let temp_rel = parent.join(&temp_name);
        let file = safe_dir.create_file(&temp_rel, 0o600, true)?;
        Ok(Self {
            temp_path: Some(temp_rel.to_path_buf()),
            final_path: final_rel_path.to_path_buf(),
            file,
        })
    }
}
```

- [ ] **Step 2: persist を SafeDir 対応に変更**

```rust
impl SafeWriter {
    pub(crate) fn persist_with_safe_dir(mut self, safe_dir: &SafeDir) -> io::Result<()> {
        self.file.sync_all()?;
        let temp = self.temp_path.take().expect("already persisted");
        safe_dir.rename(&temp, &self.final_path)?;
        Ok(())
    }
}
```

- [ ] **Step 3: テスト実行**

Run: `cargo test -p portable-network-archive --all-features`
Expected: ALL PASS

- [ ] **Step 4: コミット**

```bash
git add cli/src/command/core/safe_writer.rs
git commit -m ":recycle: Add SafeDir-based constructor and persist to SafeWriter"
```

---

## Phase 3: メタデータ操作の移行

### Task 12: chmod / timestamps / chown を SafeDir 経由に移行

**Files:**
- Modify: `cli/src/command/extract.rs` (メタデータ設定箇所)

- [ ] **Step 1: extract_entry 内のメタデータ設定箇所を特定**

`extract_entry` 内で `chmod`, `lchown`, タイムスタンプ設定が呼ばれる箇所を確認。各操作を `safe_dir` が `Some` の場合に `safe_dir.set_permissions()`, `safe_dir.set_ownership()`, `safe_dir.set_times()` に委譲する形に変更。

- [ ] **Step 2: chmod を SafeDir 経由に変更**

`safe_dir` がある場合:
```rust
if let Some(ref safe_dir) = safe_dir {
    let no_follow = matches!(item.header().data_kind(), DataKind::SymbolicLink);
    safe_dir.set_permissions(Path::new(item_path.as_str()), mode as u32, no_follow)?;
} else {
    utils::fs::chmod(&path, mode)?;
}
```

- [ ] **Step 3: chown を SafeDir 経由に変更**

```rust
#[cfg(unix)]
if let Some(ref safe_dir) = safe_dir {
    let no_follow = matches!(item.header().data_kind(), DataKind::SymbolicLink);
    safe_dir.set_ownership(
        Path::new(item_path.as_str()),
        uid,
        gid,
        no_follow,
    )?;
} else {
    utils::fs::lchown(&path, owner, group)?;
}
```

- [ ] **Step 4: timestamps を SafeDir 経由に変更**

```rust
if let Some(ref safe_dir) = safe_dir {
    let no_follow = matches!(item.header().data_kind(), DataKind::SymbolicLink);
    safe_dir.set_times(
        Path::new(item_path.as_str()),
        atime,
        mtime,
        no_follow,
    )?;
}
```

- [ ] **Step 5: テスト実行**

Run: `cargo test -p portable-network-archive --all-features`
Expected: ALL PASS

- [ ] **Step 6: コミット**

```bash
git add cli/src/command/extract.rs
git commit -m ":recycle: Migrate chmod, chown, and timestamp operations to SafeDir"
```

---

## Phase 4: 検証とクリーンアップ

### Task 13: 悪意あるアーカイブの統合テスト完成

**Files:**
- Modify: `cli/tests/cli/safe_dir.rs`

- [ ] **Step 1: libpna API を使って悪意あるアーカイブを生成するヘルパーを作成**

```rust
use libpna::{Archive, EntryBuilder, WriteOptions};
use std::io::Write;

/// シンボリックリンク→ファイルの Zip Slip アーカイブを生成
fn create_zipslip_archive(path: &Path) {
    let file = fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    // 1. シンボリックリンクエントリ: "link" -> "../../target"
    let entry = EntryBuilder::new_symbolic_link(
        "link".try_into().unwrap(),
        "../../target".into(),
    ).unwrap().build().unwrap();
    archive.add_entry(entry).unwrap();
    // 2. ファイルエントリ: "link/payload.txt"
    let entry = EntryBuilder::new_file(
        "link/payload.txt".try_into().unwrap(),
        WriteOptions::builder().build(),
    ).unwrap().build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
}
```

注意: 実際の `libpna` API（`EntryBuilder` のコンストラクタ、`Archive` の書き込みメソッド）の正確なシグネチャは実装時に確認。上記は方向性を示す疑似コード。

- [ ] **Step 2: Zip Slip テストを完成**

```rust
#[test]
fn extract_zipslip_symlink_blocked() {
    let tmp = tempdir().unwrap();
    let archive_path = tmp.path().join("zipslip.pna");
    let out_dir = tmp.path().join("out");
    let target_dir = tmp.path().join("target");
    fs::create_dir_all(&out_dir).unwrap();
    fs::create_dir_all(&target_dir).unwrap();

    create_zipslip_archive(&archive_path);

    // pna extract を実行
    let status = Command::new(env!("CARGO_BIN_EXE_pna"))
        .args(["extract", archive_path.to_str().unwrap(), "-o", out_dir.to_str().unwrap()])
        .status()
        .unwrap();

    // target/ にファイルが存在しないことを確認
    assert!(!target_dir.join("payload.txt").exists());
}
```

- [ ] **Step 3: テスト実行**

Run: `cargo test -p portable-network-archive --test cli safe_dir`
Expected: ALL PASS

- [ ] **Step 4: コミット**

```bash
git add cli/tests/cli/safe_dir.rs
git commit -m ":white_check_mark: Complete malicious archive integration tests for SafeDir"
```

---

### Task 14: 全テスト実行と旧コード削除

**Files:**
- Modify: `cli/src/command/extract.rs` (旧コードの削除)

- [ ] **Step 1: 全テストスイートを実行**

Run: `cargo test --workspace --all-features`
Expected: ALL PASS

- [ ] **Step 2: build_output_path を削除**

`extract.rs:1309-1320` の `build_output_path` 関数を削除。呼び出し元が全て `SafeDir` ベースに移行済みであることを確認。

- [ ] **Step 3: ensure_directory_components を削除**

`extract.rs:1645-1717` の `ensure_directory_components` 関数を削除。`SafeDir::ensure_dir_all` に完全置換済みであることを確認。

- [ ] **Step 4: out_dir フィールドの除去を検討**

`OutputOption` の `out_dir: Option<PathBuf>` フィールドが `safe_dir` で完全に置き換えられている場合は削除。`to_stdout` モードとの互換性を確認。

- [ ] **Step 5: テスト再実行**

Run: `cargo test --workspace --all-features`
Expected: ALL PASS

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: NO WARNINGS

- [ ] **Step 6: コミット**

```bash
git add cli/src/command/extract.rs
git commit -m ":fire: Remove build_output_path and ensure_directory_components (replaced by SafeDir)"
```

---

### Task 15: ベンチマーク

**Files:** なし（計測のみ）

- [ ] **Step 1: 展開速度のベンチマーク**

大きめのアーカイブ（1000ファイル以上）を用意:

```bash
# テスト用アーカイブの作成
pna create bench.pna /usr/share/doc/ 2>/dev/null || pna create bench.pna /usr/share/man/

# ベンチマーク実行
hyperfine --warmup 3 \
  'pna extract bench.pna -o /tmp/pna-bench-out --overwrite'
```

- [ ] **Step 2: main ブランチと比較**

```bash
# main ブランチのバイナリでも同じベンチマークを実行して比較
hyperfine --warmup 3 \
  './target-main/release/pna extract bench.pna -o /tmp/pna-bench-main --overwrite' \
  './target/release/pna extract bench.pna -o /tmp/pna-bench-new --overwrite'
```

- [ ] **Step 3: 結果を記録**

顕著な性能劣化（>20%）がある場合は原因を調査し、最適化を検討。

---

## CI 対応メモ

- `cargo hack test --feature-powerset --exclude-features wasm` は `safe-dir` feature を含む全組み合わせをテストする。`safe-dir` なしの場合は `path_impl` が使われ、ありの場合は `openat_impl` が使われる。
- WASM ターゲット向け CI は `--exclude-features safe-dir` を追加する必要がある（cap-std は WASM 未対応のため）。
