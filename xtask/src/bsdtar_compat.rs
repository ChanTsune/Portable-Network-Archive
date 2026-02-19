use clap::Parser;
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

fn materialize(root: &Path, specs: &[FileSpec]) -> io::Result<()> {
    for spec in specs {
        match spec {
            FileSpec::File {
                path,
                contents,
                mtime_epoch,
            } => {
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum FsEntry {
    File { contents: Vec<u8> },
    Dir,
    Symlink { target: PathBuf },
}

impl std::fmt::Display for FsEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsEntry::File { contents } => match std::str::from_utf8(contents) {
                Ok(s) => write!(f, "File({s:?})"),
                Err(_) => write!(f, "File({} bytes)", contents.len()),
            },
            FsEntry::Dir => write!(f, "Dir"),
            FsEntry::Symlink { target } => write!(f, "Symlink({})", target.display()),
        }
    }
}

#[derive(Debug)]
struct FsSnapshot(BTreeMap<PathBuf, FsEntry>);

impl FsSnapshot {
    fn capture(root: &Path) -> io::Result<Self> {
        let mut entries = BTreeMap::new();
        Self::walk(root, root, &mut entries)?;
        Ok(Self(entries))
    }

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

struct Diff {
    path: PathBuf,
    bsdtar: Option<FsEntry>,
    pna: Option<FsEntry>,
}

fn compare_snapshots(bsdtar: &FsSnapshot, pna: &FsSnapshot) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let all_keys: std::collections::BTreeSet<_> = bsdtar.0.keys().chain(pna.0.keys()).collect();

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

fn run_cmd(cmd: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("command failed: {:?}\nstderr: {stderr}", cmd.get_program()).into());
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
    Command::new("bsdtar")
        .arg("--version")
        .output()
        .map_err(|_| "bsdtar not found in PATH. Install libarchive.")?;
    Ok(())
}

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
        .arg("-C")
        .arg(&bsdtar_src)
        .arg(".");
    run_cmd(&mut cmd)?;

    let mut cmd = Command::new("bsdtar");
    cmd.args(["-xf", bsdtar_archive.to_str().unwrap()])
        .args(scenario.extract_options)
        .arg("-C")
        .arg(&bsdtar_dst);
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
        .arg("-C")
        .arg(&pna_src)
        .arg(".");
    run_cmd(&mut cmd)?;

    let mut cmd = Command::new(pna_bin);
    cmd.args(["experimental", "stdio", "--unstable"])
        .args(["-xf", pna_archive.to_str().unwrap()])
        .args(scenario.extract_options)
        .arg("-C")
        .arg(&pna_dst);
    run_cmd(&mut cmd)?;

    let pna_snap = FsSnapshot::capture(&pna_dst)?;

    Ok(compare_snapshots(&bsdtar_snap, &pna_snap))
}

#[derive(Parser)]
pub struct BsdtarCompatArgs {}

pub fn run(_args: BsdtarCompatArgs) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("bsdtar-compat: not yet implemented");
    Ok(())
}
