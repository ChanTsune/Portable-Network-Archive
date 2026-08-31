use crate::{
    cli::ArchiveFileArgs,
    command::{
        Command,
        core::{ArchiveSource, write_split_archive},
    },
    utils::PathWithCwd,
};
use anyhow::{Context, ensure};
use bytesize::ByteSize;
use clap::{ArgAction, Parser, ValueHint};
use pna::{Archive, MIN_SPLIT_PART_BYTES};
use std::{fs, io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SplitCommand {
    #[command(flatten)]
    archive: ArchiveFileArgs,
    #[arg(
        long,
        value_name = "BASE_PATH",
        help = "Base path used to name split archive parts",
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
    let max_file_size = usize::try_from(args.max_size.unwrap_or_else(|| ByteSize::gb(1)).as_u64())
        .context("--max-size is too large for this platform")?;
    ensure!(
        max_file_size >= MIN_SPLIT_PART_BYTES,
        "The value for --max-size must be at least {MIN_SPLIT_PART_BYTES} bytes ({}).",
        ByteSize::b(MIN_SPLIT_PART_BYTES as u64)
    );

    let source = args.archive.source();
    let source_path = match &source {
        ArchiveSource::File(path) => Some(path.as_path()),
        ArchiveSource::Stdin => None,
    };
    let requested_output = args
        .output
        .as_deref()
        .or(source_path)
        .context(
            "archive input from standard input requires --output BASE_PATH to name the split parts",
        )?
        .to_path_buf();
    let base_out_file_name = if let Some(out_dir) = args.out_dir {
        fs::create_dir_all(&out_dir)?;
        out_dir.join(requested_output.file_name().unwrap_or_default())
    } else {
        requested_output
    };

    match source {
        ArchiveSource::File(archive_path) => split_file_archive(
            &archive_path,
            &base_out_file_name,
            max_file_size,
            args.overwrite,
        ),
        ArchiveSource::Stdin => split_reader_archive(
            io::stdin().lock(),
            &base_out_file_name,
            max_file_size,
            args.overwrite,
        ),
    }
    .with_context(|| {
        format!(
            "failed to create `{}`",
            PathWithCwd::new(&base_out_file_name)
        )
    })
}

fn split_reader_archive(
    reader: impl io::Read,
    output: &std::path::Path,
    max_file_size: usize,
    overwrite: bool,
) -> anyhow::Result<()> {
    let mut archive = Archive::read_header(reader)?;
    write_split_archive(output, archive.raw_entries(), max_file_size, overwrite)
}

fn split_file_archive(
    input: &std::path::Path,
    output: &std::path::Path,
    max_file_size: usize,
    overwrite: bool,
) -> anyhow::Result<()> {
    let file = fs::File::open(input)?;
    #[cfg(not(feature = "memmap"))]
    {
        split_reader_archive(file, output, max_file_size, overwrite)
    }
    #[cfg(feature = "memmap")]
    {
        let mapped_file = crate::utils::mmap::Mmap::try_from(file)?;
        let mut archive = Archive::read_header_from_slice(&mapped_file[..])?;
        write_split_archive(
            output,
            archive.raw_entries_slice(),
            max_file_size,
            overwrite,
        )
    }
}
