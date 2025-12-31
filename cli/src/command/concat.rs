#[cfg(not(feature = "memmap"))]
use crate::command::core::run_across_archive;
#[cfg(feature = "memmap")]
use crate::command::core::run_across_archive_mem as run_across_archive;
use crate::{
    command::{Command, core::collect_split_archives},
    utils,
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::Archive;
use std::{io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    group(ArgGroup::new("archive-args").args(["files", "archives"]).required(true)),
    group(ArgGroup::new("overwrite-flag").args(["overwrite", "no_overwrite"])),
)]
pub(crate) struct ConcatCommand {
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[arg(
        long,
        help = "Do not overwrite files. This is the inverse option of --overwrite"
    )]
    no_overwrite: bool,
    #[arg(help = "Archive files to concatenate (deprecated, use --files)", value_hint = ValueHint::FilePath)]
    archives: Vec<PathBuf>,
    #[arg(short, long, help = "Archive files to concatenate", value_hint = ValueHint::FilePath)]
    files: Vec<PathBuf>,
}

impl Command for ConcatCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        concat_entry(self)
    }
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
