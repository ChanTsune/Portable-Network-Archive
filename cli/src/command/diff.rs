#[cfg(feature = "memmap")]
use crate::command::core::run_entries;
#[cfg(not(feature = "memmap"))]
use crate::command::core::run_process_archive as run_entries;
use crate::{
    cli::{FileArgs, PasswordArgs},
    command::{ask_password, core::collect_split_archives, Command},
};

use clap::Parser;
use pna::{DataKind, NormalEntry, ReadOptions};
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
    fn execute(self) -> anyhow::Result<()> {
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

fn compare_entry<T: AsRef<[u8]>>(entry: NormalEntry<T>, password: Option<&str>) -> io::Result<()> {
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
        DataKind::HardLink => (),
    }
    Ok(())
}
