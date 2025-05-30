#[cfg(not(feature = "memmap"))]
use crate::command::commons::run_across_archive;
#[cfg(feature = "memmap")]
use crate::command::commons::run_across_archive_mem as run_across_archive;
use crate::{
    cli::FileArgs,
    command::{commons::collect_split_archives, Command},
    utils,
};
use clap::Parser;
use pna::Archive;
use std::io;

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ConcatCommand {
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[command(flatten)]
    files: FileArgs,
}

impl Command for ConcatCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        concat_entry(self)
    }
}

fn concat_entry(args: ConcatCommand) -> io::Result<()> {
    if !args.overwrite && args.files.archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", args.files.archive.display()),
        ));
    }
    for item in &args.files.files {
        if !utils::fs::is_pna(item)? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{item} is not a pna file"),
            ));
        }
    }
    let file = utils::fs::file_create(&args.files.archive, args.overwrite)?;
    let mut archive = Archive::write_header(file)?;

    for item in &args.files.files {
        let archives = collect_split_archives(item)?;
        run_across_archive(archives, |reader| {
            #[cfg(feature = "memmap")]
            for entry in reader.raw_entries_slice() {
                archive.add_entry(entry?)?;
            }
            #[cfg(not(feature = "memmap"))]
            for entry in reader.raw_entries() {
                archive.add_entry(entry?)?;
            }
            Ok(())
        })?;
    }
    archive.finalize()?;
    Ok(())
}
