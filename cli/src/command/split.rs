use crate::{
    command::{
        Command,
        core::{MIN_SPLIT_PART_BYTES, write_split_archive},
    },
    utils::PathPartExt,
};
use anyhow::ensure;
use bytesize::ByteSize;
use clap::{ArgGroup, Parser, ValueHint};
use pna::Archive;
use std::{fs, io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    group(ArgGroup::new("archive_arg").args(["file", "archive"]).required(true)),
    group(ArgGroup::new("overwrite-flag").args(["overwrite", "no_overwrite"]))
)]
pub(crate) struct SplitCommand {
    #[arg(short = 'f', long = "file", help = "Archive file path", value_hint = ValueHint::FilePath)]
    file: Option<PathBuf>,
    #[arg(value_hint = ValueHint::FilePath, hide = true)]
    pub(crate) archive: Option<PathBuf>,
    #[arg(long, help = "Output directory for split archives", value_hint = ValueHint::DirPath)]
    out_dir: Option<PathBuf>,
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[arg(
        long,
        help = "Do not overwrite files. This is the inverse option of --overwrite"
    )]
    no_overwrite: bool,
    #[arg(
        long,
        value_name = "size",
        help = "Maximum size in bytes of split archive (minimum 64B)"
    )]
    pub(crate) max_size: Option<ByteSize>,
}

impl Command for SplitCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        split_archive(self)
    }
}

fn split_archive(args: SplitCommand) -> anyhow::Result<()> {
    let archive_path = match (args.file, args.archive) {
        (Some(f), _) => f,
        (None, Some(a)) => {
            log::warn!("positional `archive` is deprecated, use `--file` instead");
            a
        }
        _ => unreachable!("required by ArgGroup"),
    };
    let max_file_size = args.max_size.unwrap_or_else(|| ByteSize::gb(1)).as_u64() as usize;
    ensure!(
        max_file_size >= MIN_SPLIT_PART_BYTES,
        "The value for --max-size must be at least {MIN_SPLIT_PART_BYTES} bytes ({}).",
        ByteSize::b(MIN_SPLIT_PART_BYTES as u64)
    );
    let read_file = fs::File::open(&archive_path)?;
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
        out_dir.join(archive_path.file_name().unwrap_or_default())
    } else {
        archive_path.clone()
    };
    let name = base_out_file_name.with_part(1);
    if !args.overwrite && name.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", name.display()),
        )
        .into());
    }
    write_split_archive(base_out_file_name, entries, max_file_size, args.overwrite)
}
