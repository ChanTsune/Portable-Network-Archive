use crate::{
    cli::{FileArgs, Verbosity},
    command::Command,
    utils,
};
use clap::Parser;
use pna::Archive;
use std::{fs, io};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ConcatCommand {
    #[command(flatten)]
    files: FileArgs,
}

impl Command for ConcatCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        concat_entry(self, verbosity)
    }
}

fn concat_entry(args: ConcatCommand, _verbosity: Verbosity) -> io::Result<()> {
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
        let file = fs::File::open(item)?;
        let mut reader = Archive::read_header(file)?;
        for entry in reader.raw_entries() {
            archive.add_entry(entry?)?;
        }
    }
    archive.finalize()?;
    Ok(())
}
