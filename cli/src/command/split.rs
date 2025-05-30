use crate::{
    command::{commons::write_split_archive, Command},
    utils::PathPartExt,
};
use bytesize::ByteSize;
use clap::{Parser, ValueHint};
use pna::Archive;
use std::{fs, io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SplitCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    pub(crate) archive: PathBuf,
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub(crate) out_dir: Option<PathBuf>,
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(
        long,
        value_name = "size",
        help = "Maximum size in bytes of split archive"
    )]
    pub(crate) max_size: Option<ByteSize>,
}

impl Command for SplitCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        split_archive(self)
    }
}

fn split_archive(args: SplitCommand) -> io::Result<()> {
    let read_file = fs::File::open(&args.archive)?;
    #[cfg(not(feature = "memmap"))]
    let mut read_archive = Archive::read_header(read_file)?;
    #[cfg(not(feature = "memmap"))]
    let entries = read_archive.raw_entries();
    #[cfg(feature = "memmap")]
    let mapped_file = crate::utils::mmap::Mmap::try_from(read_file)?;
    #[cfg(feature = "memmap")]
    let mut read_archive = Archive::read_header_from_slice(&mapped_file[..])?;
    #[cfg(feature = "memmap")]
    let entries = read_archive.raw_entries_slice();

    let base_out_file_name = if let Some(out_dir) = args.out_dir {
        fs::create_dir_all(&out_dir)?;
        out_dir.join(args.archive.file_name().unwrap_or_default())
    } else {
        args.archive.clone()
    };
    let name = base_out_file_name.with_part(1).unwrap();
    if !args.overwrite && name.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", name.display()),
        ));
    }
    let max_file_size = args.max_size.unwrap_or_else(|| ByteSize::gb(1)).as_u64() as usize;

    write_split_archive(base_out_file_name, entries, max_file_size, args.overwrite)
}
