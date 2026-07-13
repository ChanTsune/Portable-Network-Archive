# bsdtar Oracle Compatibility Testing — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `cargo xtask bsdtar-compat` that runs extraction scenarios through both bsdtar and pna stdio, comparing filesystem results.

**Architecture:** Single new module `xtask/src/bsdtar_compat.rs` added as a subcommand. Scenarios are static Rust data. For each scenario: materialize source files → archive → set up pre-extraction state → extract → snapshot filesystem → compare snapshots between bsdtar and pna.

**Tech Stack:** Rust, clap (existing xtask dependency), std::process::Command, tempfile for isolation.

---

### Task 1: Add subcommand scaffolding

**Files:**
- Modify: `xtask/Cargo.toml` (add `tempfile` dependency)
- Modify: `xtask/src/main.rs` (add `BsdtarCompat` variant + dispatch)
- Create: `xtask/src/bsdtar_compat.rs` (empty handler)

**Step 1: Add tempfile dependency**

In `xtask/Cargo.toml`, add to `[dependencies]`:
```toml
tempfile = "3"
```

**Step 2: Create `xtask/src/bsdtar_compat.rs` with stub**

```rust
use clap::Parser;

#[derive(Parser)]
pub struct BsdtarCompatArgs {}

pub fn run(_args: BsdtarCompatArgs) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("bsdtar-compat: not yet implemented");
    Ok(())
}
```

**Step 3: Wire into `main.rs`**

Add `mod bsdtar_compat;` at top.

Add variant to `Command` enum:
```rust
/// Verify extraction behavior matches bsdtar
BsdtarCompat(bsdtar_compat::BsdtarCompatArgs),
```

Add match arm in `run()`:
```rust
Command::BsdtarCompat(args) => bsdtar_compat::run(args),
```

**Step 4: Verify it compiles and runs**

Run: `cargo xtask bsdtar-compat`
Expected: prints "bsdtar-compat: not yet implemented"

**Step 5: Commit**

```
:construction: Add bsdtar-compat xtask subcommand scaffold
```

---

### Task 2: Implement FileSpec and materialize()

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs`

**Step 1: Define FileSpec and Scenario types**

Add at top of `bsdtar_compat.rs`:

```rust
use std::collections::BTreeMap;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use std::{fs, io};

pub enum FileSpec {
    File {
        path: &'static str,
        contents: &'static [u8],
        mtime_epoch: Option<i64>,
    },
    Dir {
        path: &'static str,
    },
    Symlink {
        path: &'static str,
        target: &'static str,
    },
    Hardlink {
        path: &'static str,
        target: &'static str,
    },
}

pub struct Scenario {
    pub name: &'static str,
    pub source_files: &'static [FileSpec],
    pub pre_existing: &'static [FileSpec],
    pub create_options: &'static [&'static str],
    pub extract_options: &'static [&'static str],
}
```

**Step 2: Implement materialize()**

```rust
fn materialize(root: &Path, specs: &[FileSpec]) -> io::Result<()> {
    for spec in specs {
        match spec {
            FileSpec::File { path, contents, mtime_epoch } => {
                let full = root.join(path);
                if let Some(parent) = full.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full, contents)?;
                if let Some(epoch) = mtime_epoch {
                    let time = if *epoch >= 0 {
                        SystemTime::UNIX_EPOCH + Duration::from_secs(*epoch as u64)
                    } else {
                        SystemTime::UNIX_EPOCH - Duration::from_secs(epoch.unsigned_abs())
                    };
                    let file = fs::File::options().write(true).open(&full)?;
                    file.set_modified(time)?;
                }
            }
            FileSpec::Dir { path } => {
                fs::create_dir_all(root.join(path))?;
            }
            FileSpec::Symlink { path, target } => {
                let full = root.join(path);
                if let Some(parent) = full.parent() {
                    fs::create_dir_all(parent)?;
                }
                unix_fs::symlink(target, &full)?;
            }
            FileSpec::Hardlink { path, target } => {
                let full = root.join(path);
                if let Some(parent) = full.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::hard_link(root.join(target), &full)?;
            }
        }
    }
    Ok(())
}
```

**Step 3: Verify compiles**

Run: `cargo build -p xtask`
Expected: success

**Step 4: Commit**

```
:construction: Add FileSpec, Scenario, and materialize() for bsdtar-compat
```

---

### Task 3: Implement FsSnapshot capture and comparison

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs`

**Step 1: Define FsSnapshot types**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEntry {
    File { contents: Vec<u8> },
    Dir,
    Symlink { target: PathBuf },
}

impl std::fmt::Display for FsEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsEntry::File { contents } => {
                match std::str::from_utf8(contents) {
                    Ok(s) => write!(f, "File({s:?})"),
                    Err(_) => write!(f, "File({} bytes)", contents.len()),
                }
            }
            FsEntry::Dir => write!(f, "Dir"),
            FsEntry::Symlink { target } => write!(f, "Symlink({})", target.display()),
        }
    }
}

#[derive(Debug)]
pub struct FsSnapshot(BTreeMap<PathBuf, FsEntry>);

impl FsSnapshot {
    fn capture(root: &Path) -> io::Result<Self> {
        let mut entries = BTreeMap::new();
        Self::walk(root, root, &mut entries)?;
        Ok(Self(entries))
    }

    fn walk(
        root: &Path,
        dir: &Path,
        entries: &mut BTreeMap<PathBuf, FsEntry>,
    ) -> io::Result<()> {
        let mut dir_entries: Vec<_> = fs::read_dir(dir)?
            .collect::<Result<Vec<_>, _>>()?;
        dir_entries.sort_by_key(|e| e.file_name());

        for entry in dir_entries {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            let meta = fs::symlink_metadata(&path)?;

            if meta.is_symlink() {
                let target = fs::read_link(&path)?;
                entries.insert(rel, FsEntry::Symlink { target });
            } else if meta.is_dir() {
                entries.insert(rel.clone(), FsEntry::Dir);
                Self::walk(root, &path, entries)?;
            } else {
                let contents = fs::read(&path)?;
                entries.insert(rel, FsEntry::File { contents });
            }
        }
        Ok(())
    }
}
```

**Step 2: Implement comparison**

```rust
pub struct Diff {
    pub path: PathBuf,
    pub bsdtar: Option<FsEntry>,
    pub pna: Option<FsEntry>,
}

fn compare_snapshots(bsdtar: &FsSnapshot, pna: &FsSnapshot) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let all_keys: std::collections::BTreeSet<_> =
        bsdtar.0.keys().chain(pna.0.keys()).collect();

    for key in all_keys {
        let b = bsdtar.0.get(key);
        let p = pna.0.get(key);
        if b != p {
            diffs.push(Diff {
                path: key.clone(),
                bsdtar: b.cloned(),
                pna: p.cloned(),
            });
        }
    }
    diffs
}
```

**Step 3: Verify compiles**

Run: `cargo build -p xtask`
Expected: success

**Step 4: Commit**

```
:construction: Add FsSnapshot capture and comparison for bsdtar-compat
```

---

### Task 4: Implement the scenario runner

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs`

**Step 1: Add helper to run commands**

```rust
fn run_cmd(cmd: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "command failed: {:?}\nstderr: {stderr}",
            cmd.get_program()
        ).into());
    }
    Ok(())
}

fn find_pna_binary() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let status = Command::new("cargo")
        .args(["build", "-p", "portable-network-archive"])
        .status()?;
    if !status.success() {
        return Err("failed to build pna binary".into());
    }
    let path = PathBuf::from("target/debug/pna");
    if !path.exists() {
        return Err(format!("pna binary not found at {}", path.display()).into());
    }
    Ok(fs::canonicalize(path)?)
}

fn check_bsdtar() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("bsdtar").arg("--version").output()
        .map_err(|_| "bsdtar not found in PATH. Install libarchive.")?;
    Ok(())
}
```

**Step 2: Implement run_scenario()**

```rust
fn run_scenario(
    scenario: &Scenario,
    pna_bin: &Path,
) -> Result<Vec<Diff>, Box<dyn std::error::Error>> {
    let work = tempfile::tempdir()?;
    let work = work.path();

    // --- bsdtar side ---
    let bsdtar_src = work.join("bsdtar_src");
    let bsdtar_dst = work.join("bsdtar_dst");
    let bsdtar_archive = work.join("archive.tar");
    fs::create_dir_all(&bsdtar_src)?;
    fs::create_dir_all(&bsdtar_dst)?;

    materialize(&bsdtar_src, scenario.source_files)?;
    materialize(&bsdtar_dst, scenario.pre_existing)?;

    let mut cmd = Command::new("bsdtar");
    cmd.args(["-cf", bsdtar_archive.to_str().unwrap()])
        .args(scenario.create_options)
        .arg("-C").arg(&bsdtar_src).arg(".");
    run_cmd(&mut cmd)?;

    let mut cmd = Command::new("bsdtar");
    cmd.args(["-xf", bsdtar_archive.to_str().unwrap()])
        .args(scenario.extract_options)
        .arg("-C").arg(&bsdtar_dst);
    run_cmd(&mut cmd)?;

    let bsdtar_snap = FsSnapshot::capture(&bsdtar_dst)?;

    // --- pna side ---
    let pna_src = work.join("pna_src");
    let pna_dst = work.join("pna_dst");
    let pna_archive = work.join("archive.pna");
    fs::create_dir_all(&pna_src)?;
    fs::create_dir_all(&pna_dst)?;

    materialize(&pna_src, scenario.source_files)?;
    materialize(&pna_dst, scenario.pre_existing)?;

    let mut cmd = Command::new(pna_bin);
    cmd.args(["experimental", "stdio", "--unstable"])
        .args(["-cf", pna_archive.to_str().unwrap()])
        .args(scenario.create_options)
        .arg("-C").arg(&pna_src).arg(".");
    run_cmd(&mut cmd)?;

    let mut cmd = Command::new(pna_bin);
    cmd.args(["experimental", "stdio", "--unstable"])
        .args(["-xf", pna_archive.to_str().unwrap()])
        .args(scenario.extract_options)
        .arg("-C").arg(&pna_dst);
    run_cmd(&mut cmd)?;

    let pna_snap = FsSnapshot::capture(&pna_dst)?;

    Ok(compare_snapshots(&bsdtar_snap, &pna_snap))
}
```

**Step 3: Verify compiles**

Run: `cargo build -p xtask`
Expected: success

**Step 4: Commit**

```
:construction: Add scenario runner for bsdtar-compat
```

---

### Task 5: Define scenarios and wire up main entry point

**Files:**
- Modify: `xtask/src/bsdtar_compat.rs`

**Step 1: Define scenarios**

```rust
static SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "baseline",
        source_files: &[
            FileSpec::File { path: "file.txt", contents: b"hello", mtime_epoch: None },
            FileSpec::Dir { path: "dir" },
            FileSpec::File { path: "dir/nested.txt", contents: b"nested", mtime_epoch: None },
        ],
        pre_existing: &[],
        create_options: &[],
        extract_options: &[],
    },
    Scenario {
        name: "existing_file_overwrite",
        source_files: &[
            FileSpec::File { path: "file.txt", contents: b"from_archive", mtime_epoch: None },
        ],
        pre_existing: &[
            FileSpec::File { path: "file.txt", contents: b"existing", mtime_epoch: None },
        ],
        create_options: &[],
        extract_options: &[],
    },
    Scenario {
        name: "unlink_basic",
        source_files: &[
            FileSpec::File { path: "file.txt", contents: b"from_archive", mtime_epoch: None },
        ],
        pre_existing: &[
            FileSpec::File { path: "file.txt", contents: b"existing", mtime_epoch: None },
        ],
        create_options: &[],
        extract_options: &["-U"],
    },
    Scenario {
        name: "unlink_symlink_file",
        source_files: &[
            FileSpec::File { path: "file.txt", contents: b"from_archive", mtime_epoch: None },
        ],
        pre_existing: &[
            FileSpec::Symlink { path: "file.txt", target: "/dev/null" },
        ],
        create_options: &[],
        extract_options: &["-U"],
    },
    Scenario {
        name: "unlink_symlink_parent",
        source_files: &[
            FileSpec::File { path: "dir/file.txt", contents: b"payload", mtime_epoch: None },
        ],
        pre_existing: &[
            FileSpec::Symlink { path: "dir", target: "/tmp" },
        ],
        create_options: &[],
        extract_options: &["-U"],
    },
    Scenario {
        name: "unlink_keep_old",
        source_files: &[
            FileSpec::File { path: "file.txt", contents: b"from_archive", mtime_epoch: None },
        ],
        pre_existing: &[
            FileSpec::File { path: "file.txt", contents: b"existing", mtime_epoch: None },
        ],
        create_options: &[],
        extract_options: &["-U", "-k"],
    },
    Scenario {
        name: "unlink_keep_newer",
        source_files: &[
            FileSpec::File { path: "file.txt", contents: b"from_archive", mtime_epoch: Some(1) },
        ],
        pre_existing: &[
            FileSpec::File {
                path: "file.txt",
                contents: b"newer_on_disk",
                mtime_epoch: Some(2000000000),
            },
        ],
        create_options: &[],
        extract_options: &["-U", "--keep-newer-files"],
    },
    Scenario {
        name: "keep_old_without_unlink",
        source_files: &[
            FileSpec::File { path: "file.txt", contents: b"from_archive", mtime_epoch: None },
        ],
        pre_existing: &[
            FileSpec::File { path: "file.txt", contents: b"existing", mtime_epoch: None },
        ],
        create_options: &[],
        extract_options: &["-k"],
    },
    Scenario {
        name: "keep_newer_preserves",
        source_files: &[
            FileSpec::File { path: "file.txt", contents: b"from_archive", mtime_epoch: Some(1) },
        ],
        pre_existing: &[
            FileSpec::File {
                path: "file.txt",
                contents: b"newer_on_disk",
                mtime_epoch: Some(2000000000),
            },
        ],
        create_options: &[],
        extract_options: &["--keep-newer-files"],
    },
    Scenario {
        name: "keep_newer_overwrites",
        source_files: &[
            FileSpec::File { path: "file.txt", contents: b"from_archive", mtime_epoch: Some(2000000000) },
        ],
        pre_existing: &[
            FileSpec::File {
                path: "file.txt",
                contents: b"old_on_disk",
                mtime_epoch: Some(1),
            },
        ],
        create_options: &[],
        extract_options: &["--keep-newer-files"],
    },
    Scenario {
        name: "follow_symlink_P",
        source_files: &[
            FileSpec::File { path: "dir/file.txt", contents: b"payload", mtime_epoch: None },
        ],
        pre_existing: &[
            FileSpec::Dir { path: "real_dir" },
            FileSpec::Symlink { path: "dir", target: "real_dir" },
        ],
        create_options: &[],
        extract_options: &["-P"],
    },
    Scenario {
        name: "unlink_follow_PU",
        source_files: &[
            FileSpec::File { path: "dir/file.txt", contents: b"payload", mtime_epoch: None },
        ],
        pre_existing: &[
            FileSpec::Dir { path: "real_dir" },
            FileSpec::Symlink { path: "dir", target: "real_dir" },
        ],
        create_options: &[],
        extract_options: &["-P", "-U"],
    },
];
```

**Step 2: Implement `run()` entry point**

```rust
pub fn run(_args: BsdtarCompatArgs) -> Result<(), Box<dyn std::error::Error>> {
    check_bsdtar()?;
    let pna_bin = find_pna_binary()?;

    let total = SCENARIOS.len();
    eprintln!("bsdtar-compat: running {total} scenarios");

    let mut passed = 0;
    let mut failed = 0;

    for scenario in SCENARIOS {
        match run_scenario(scenario, &pna_bin) {
            Ok(diffs) if diffs.is_empty() => {
                eprintln!("[PASS] {}", scenario.name);
                passed += 1;
            }
            Ok(diffs) => {
                eprintln!("[FAIL] {}", scenario.name);
                for diff in &diffs {
                    eprintln!("  diff at {}:", diff.path.display());
                    match &diff.bsdtar {
                        Some(e) => eprintln!("    bsdtar: {e}"),
                        None    => eprintln!("    bsdtar: (absent)"),
                    }
                    match &diff.pna {
                        Some(e) => eprintln!("    pna:    {e}"),
                        None    => eprintln!("    pna:    (absent)"),
                    }
                }
                failed += 1;
            }
            Err(e) => {
                eprintln!("[ERROR] {}: {e}", scenario.name);
                failed += 1;
            }
        }
    }

    eprintln!("---");
    eprintln!("{total} scenarios: {passed} passed, {failed} failed");

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}
```

**Step 3: Run and verify all scenarios pass**

Run: `cargo xtask bsdtar-compat`
Expected: 12 scenarios: 12 passed, 0 failed

**Step 4: Commit**

```
:sparkles: Add bsdtar oracle compatibility testing via xtask
```

---

### Task 6: Verify by temporarily breaking extract.rs

**Purpose:** Confirm the oracle actually catches regressions.

**Step 1: Temporarily revert the -U -k fix in extract.rs**

Change line 1073 from:
```rust
OverwriteStrategy::KeepOlder if !unlink_first => {
```
back to:
```rust
OverwriteStrategy::KeepOlder => {
```

**Step 2: Run xtask and confirm it detects the difference**

Run: `cargo xtask bsdtar-compat`
Expected: `[FAIL] unlink_keep_old` with diff showing the behavioral difference

**Step 3: Restore the fix**

Undo the revert to restore `if !unlink_first`.

**Step 4: Run again to confirm all pass**

Run: `cargo xtask bsdtar-compat`
Expected: 12 scenarios: 12 passed, 0 failed

*Do not commit — this was a verification step only.*
