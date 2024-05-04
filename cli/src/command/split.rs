use crate::{
    cli::Verbosity,
    command::{commons::write_split_archive, Command},
    utils::PathPartExt,
};
use bytesize::ByteSize;
use clap::{Parser, ValueHint};
use pna::Archive;
use std::{fs, fs::File, io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SplitCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    pub(crate) archive: PathBuf,
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub(crate) out_dir: Option<PathBuf>,
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Maximum size of split archive")]
    pub(crate) max_size: Option<ByteSize>,
}

impl Command for SplitCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        split_archive(self, verbosity)
    }
}

fn split_archive(args: SplitCommand, _verbosity: Verbosity) -> io::Result<()> {
    let read_file = File::open(&args.archive)?;
    let base_out_file_name = if let Some(out_dir) = args.out_dir {
        fs::create_dir_all(&out_dir)?;
        out_dir.join(args.archive.file_name().unwrap_or_default())
    } else {
        args.archive.clone()
    };
    let mut read_archive = Archive::read_header(read_file)?;

    let name = base_out_file_name.with_part(1).unwrap();
    if !args.overwrite && name.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is already exists", name.display()),
        ));
    }
    let max_file_size = args.max_size.unwrap_or_else(|| ByteSize::gb(1)).as_u64() as usize;

    write_split_archive(
        base_out_file_name,
        read_archive.raw_entries(),
        max_file_size,
    )
}
