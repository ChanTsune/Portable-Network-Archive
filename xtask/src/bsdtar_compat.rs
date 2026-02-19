use clap::Parser;
use std::os::unix::fs as unix_fs;
use std::path::Path;
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

#[derive(Parser)]
pub struct BsdtarCompatArgs {}

pub fn run(_args: BsdtarCompatArgs) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("bsdtar-compat: not yet implemented");
    Ok(())
}
