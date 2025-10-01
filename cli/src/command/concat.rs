#[cfg(not(feature = "memmap"))]
use crate::command::commons::run_across_archive;
#[cfg(feature = "memmap")]
use crate::command::commons::run_across_archive_mem as run_across_archive;
use crate::{
    command::{append::open_archive_then_seek_to_end, commons::collect_split_archives, Command},
    utils,
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::Archive;
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub(crate) struct ConcatFromStdioArgs {
    pub(crate) overwrite: bool,
    pub(crate) target: PathBuf,
    pub(crate) sources: Vec<PathBuf>,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("archive-args").args(["files", "archives"]).required(true)))]
pub(crate) struct ConcatCommand {
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[arg(value_hint = ValueHint::FilePath)]
    archives: Vec<PathBuf>,
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    files: Vec<PathBuf>,
}

impl Command for ConcatCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        concat_entry(self)
    }
}

pub(crate) fn run_concat_from_stdio(args: ConcatFromStdioArgs) -> anyhow::Result<()> {
    let ConcatFromStdioArgs {
        overwrite,
        target,
        sources,
    } = args;

    let mut files = Vec::with_capacity(1 + sources.len());
    files.push(target);
    files.extend(sources);

    ConcatCommand {
        overwrite,
        archives: Vec::new(),
        files,
    }
    .execute()
}

pub(crate) fn append_archives_into_existing(
    archive_path: &Path,
    sources: &[PathBuf],
) -> anyhow::Result<()> {
    for source in sources {
        let archives = collect_split_archives(source)?;
        let mut archive = open_archive_then_seek_to_end(archive_path)?;
        #[cfg(feature = "memmap")]
        {
            let mmaps = archives
                .into_iter()
                .map(utils::mmap::Mmap::try_from)
                .collect::<io::Result<Vec<_>>>()?;
            let iter = mmaps.iter().map(|m| m.as_ref());
            run_across_archive(iter, |reader| {
                for entry in reader.raw_entries_slice() {
                    archive.add_entry(entry?)?;
                }
                Ok(())
            })?;
        }
        #[cfg(not(feature = "memmap"))]
        {
            run_across_archive(archives, |reader| {
                for entry in reader.raw_entries() {
                    archive.add_entry(entry?)?;
                }
                Ok(())
            })?;
        }
        archive.finalize()?;
    }
    Ok(())
}

fn concat_entry(args: ConcatCommand) -> anyhow::Result<()> {
    let mut archives = if args.files.is_empty() {
        if !args.archives.is_empty() {
            log::warn!("positional `archive` is deprecated, use `--file` instead");
        }
        args.archives
    } else {
        args.files
    };
    let archive = archives.remove(0);
    if !args.overwrite && archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", archive.display()),
        )
        .into());
    }
    for item in &archives {
        if !utils::fs::is_pna(item)? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} is not a pna file", item.display()),
            )
            .into());
        }
    }
    let file = utils::fs::file_create(&archive, args.overwrite)?;
    let mut archive = Archive::write_header(file)?;

    for item in &archives {
        let archives = collect_split_archives(item)?;
        #[cfg(feature = "memmap")]
        {
            let mmaps = archives
                .into_iter()
                .map(utils::mmap::Mmap::try_from)
                .collect::<io::Result<Vec<_>>>()?;
            let archives = mmaps.iter().map(|m| m.as_ref());
            run_across_archive(archives, |reader| {
                for entry in reader.raw_entries_slice() {
                    archive.add_entry(entry?)?;
                }
                Ok(())
            })?;
        }
        #[cfg(not(feature = "memmap"))]
        {
            run_across_archive(archives, |reader| {
                for entry in reader.raw_entries() {
                    archive.add_entry(entry?)?;
                }
                Ok(())
            })?;
        }
    }
    archive.finalize()?;
    Ok(())
}
