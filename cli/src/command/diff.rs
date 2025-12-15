#[cfg(feature = "memmap")]
use crate::command::core::run_entries;
#[cfg(not(feature = "memmap"))]
use crate::command::core::run_process_archive as run_entries;
use crate::{
    cli::{FileArgs, PasswordArgs},
    command::{Command, ask_password, core::collect_split_archives},
};

use clap::Parser;
use pna::{DataKind, EntryReference, NormalEntry, ReadOptions};
use std::{
    fs,
    io::{self, prelude::*},
    path::Path,
};

#[derive(Parser, Clone, Debug)]
pub(crate) struct DiffCommand {
    #[command(flatten)]
    pub(crate) file: FileArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
}

impl Command for DiffCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        diff_archive(self)
    }
}

fn diff_archive(args: DiffCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let archives = collect_split_archives(&args.file.archive)?;
    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<std::io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());
    run_entries(
        archives,
        || password.as_deref(),
        |entry| compare_entry(entry?, password.as_deref()),
    )?;
    Ok(())
}

fn compare_entry<T: AsRef<[u8]>>(entry: NormalEntry<T>, password: Option<&[u8]>) -> io::Result<()> {
    let data_kind = entry.header().data_kind();
    let path = entry.header().path();
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            match data_kind {
                DataKind::File => println!("Missing file: {path}"),
                DataKind::Directory => println!("Missing directory: {path}"),
                DataKind::SymbolicLink => println!("Missing symbolic link: {path}"),
                DataKind::HardLink => println!("Missing hard link: {path}"),
            }
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    match data_kind {
        DataKind::File if meta.is_file() => match fs::read(path) {
            Ok(data) => {
                let mut reader = entry.reader(ReadOptions::with_password(password))?;
                let mut buf = Vec::new();
                reader.read_to_end(&mut buf)?;
                if buf != data {
                    println!("Different file content: {path}");
                }
            }
            Err(e) => return Err(e),
        },
        DataKind::Directory if meta.is_dir() => (),
        DataKind::SymbolicLink if meta.is_symlink() => match fs::read_link(path) {
            Ok(link) => {
                let mut reader = entry.reader(ReadOptions::with_password(password))?;
                let mut link_str = String::new();
                reader.read_to_string(&mut link_str)?;
                if link.as_path() != Path::new(&link_str) {
                    println!("Different symlink: {path}");
                }
            }
            Err(e) => return Err(e),
        },
        DataKind::File | DataKind::Directory | DataKind::SymbolicLink => {
            println!("Mismatch file type: {path}")
        }
        DataKind::HardLink if meta.is_file() => {
            compare_hard_link(&entry, password, path)?;
        }
        DataKind::HardLink => {
            println!("Mismatch file type: {path}")
        }
    }
    Ok(())
}

fn compare_hard_link<T: AsRef<[u8]>>(
    entry: &NormalEntry<T>,
    password: Option<&[u8]>,
    path: &pna::EntryName,
) -> io::Result<()> {
    let reader = entry.reader(ReadOptions::with_password(password))?;
    let link_target = io::read_to_string(reader)?;
    let link_target = EntryReference::from_utf8_preserve_root(&link_target).sanitize();

    match fs::symlink_metadata(link_target.as_path()) {
        Ok(target_meta) => {
            let link_meta = fs::symlink_metadata(path)?;
            if !same_file(&link_meta, &target_meta) {
                println!("Not linked to {}: {path}", link_target.display());
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            // Target file doesn't exist - the hard link cannot be verified
            // This is a difference because the archive expects a hard link to this target
            println!("Not linked to {}: {path}", link_target.display());
        }
        Err(e) => return Err(e),
    }
    Ok(())
}

/// Check if two metadata entries refer to the same file (same inode on Unix).
#[cfg(unix)]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino()
}

/// Check if two metadata entries refer to the same file (using file index on Windows).
#[cfg(windows)]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    // On Windows, we compare volume serial number and file index
    // Note: file_index() returns Option<u64>, so we need to handle that
    match (a.volume_serial_number(), b.volume_serial_number()) {
        (Some(vol_a), Some(vol_b)) if vol_a == vol_b => {
            a.file_index() == b.file_index()
        }
        _ => false,
    }
}

/// Fallback for platforms without inode support (e.g., WASI).
/// We cannot reliably verify hard links, so we skip the check.
#[cfg(not(any(unix, windows)))]
fn same_file(_a: &fs::Metadata, _b: &fs::Metadata) -> bool {
    // Cannot verify hard links on this platform
    true
}
