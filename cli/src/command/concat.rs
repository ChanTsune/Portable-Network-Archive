#[cfg(feature = "memmap")]
use crate::command::commons::run_across_archive_mem;
#[cfg(not(feature = "memmap"))]
use crate::command::commons::{run_across_archive, PathArchiveProvider};
use crate::{cli::FileArgs, command::Command, utils};
use clap::Parser;
use pna::Archive;
use std::{fs, io};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ConcatCommand {
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[command(flatten)]
    files: FileArgs,
}

impl Command for ConcatCommand {
    fn execute(self) -> io::Result<()> {
        concat_entry(self)
    }
}

fn concat_entry(args: ConcatCommand) -> io::Result<()> {
    if !args.overwrite && args.files.archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is already exists", args.files.archive.display()),
        ));
    }
    for item in &args.files.files {
        if !utils::fs::is_pna(item)? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} is not a pna file", item),
            ));
        }
    }
    let file = fs::File::create(&args.files.archive)?;
    let mut archive = Archive::write_header(file)?;

    for item in &args.files.files {
        #[cfg(feature = "memmap")]
        run_across_archive_mem(item, |reader| {
            for entry in reader.raw_entries_slice() {
                archive.add_entry(entry?)?;
            }
            Ok(())
        })?;
        #[cfg(not(feature = "memmap"))]
        run_across_archive(PathArchiveProvider::new(item.as_ref()), |reader| {
            for entry in reader.raw_entries() {
                archive.add_entry(entry?)?;
            }
            Ok(())
        })?;
    }
    archive.finalize()?;
    Ok(())
}
