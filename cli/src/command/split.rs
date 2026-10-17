use crate::{
    cli::ArchiveFileArgs,
    command::{Command, core::write_split_archive},
    utils::PathWithCwd,
};
use anyhow::{Context, ensure};
use bytesize::ByteSize;
use clap::{ArgAction, Parser, ValueHint};
use pna::{Archive, MIN_SPLIT_PART_BYTES};
use std::{fs, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SplitCommand {
    #[command(flatten)]
    archive: ArchiveFileArgs,
    #[arg(
        long,
        value_name = "BASE_PATH",
        help = "Base path used to name split archive parts. Relative paths resolve under --out-dir when given",
        value_hint = ValueHint::FilePath
    )]
    output: Option<PathBuf>,
    #[arg(long, value_name = "DIRECTORY", help = "Output directory for split archives", value_hint = ValueHint::DirPath)]
    out_dir: Option<PathBuf>,
    #[arg(long, conflicts_with = "no_overwrite", help = "Overwrite file")]
    overwrite: bool,
    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Do not overwrite files. This is the inverse option of --overwrite"
    )]
    no_overwrite: (),
    #[arg(
        long,
        value_name = "size",
        help = "Maximum size in bytes of split archive (minimum 64B)"
    )]
    pub(crate) max_size: Option<ByteSize>,
}

impl Command for SplitCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        split_archive(self)
    }
}

#[hooq::hooq(anyhow)]
fn split_archive(args: SplitCommand) -> anyhow::Result<()> {
    let archive_path = args.archive.require_file()?;
    let max_file_size = usize::try_from(args.max_size.unwrap_or_else(|| ByteSize::gb(1)).as_u64())
        .context("--max-size is too large for this platform")?;
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

    let base_out_file_name = match (args.out_dir, args.output) {
        (Some(out_dir), Some(output)) => out_dir.join(output),
        (Some(out_dir), None) => out_dir.join(archive_path.file_name().unwrap_or_default()),
        (None, Some(output)) => output,
        (None, None) => archive_path,
    };
    if let Some(parent) = base_out_file_name.parent() {
        fs::create_dir_all(parent)?;
    }
    write_split_archive(&base_out_file_name, entries, max_file_size, args.overwrite).with_context(
        || {
            format!(
                "failed to create `{}`",
                PathWithCwd::new(&base_out_file_name)
            )
        },
    )
}
