use clap::Parser;
use std::collections::BTreeMap;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
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

#[derive(Parser)]
pub struct BsdtarCompatArgs {}

pub fn run(_args: BsdtarCompatArgs) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("bsdtar-compat: not yet implemented");
    Ok(())
}
