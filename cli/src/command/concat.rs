#[cfg(feature = "memmap")]
use crate::command::core::run_across_archive_bytes;
#[cfg(not(feature = "memmap"))]
use crate::command::core::run_across_archive_readers;
use crate::{
    command::{
        Command,
        core::{
            Umask,
            archive_destination::{SinkConsumer, resolve_create_destination},
            collect_split_archives,
        },
    },
    utils,
};
use anyhow::ensure;
use clap::{ArgAction, Parser, ValueHint};
use pna::Archive;
use std::io;
use std::path::PathBuf;

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ConcatCommand {
    #[arg(
        short = 'f',
        long = "file",
        value_name = "INPUT",
        help = "Archive input; repeat for multiple archives or omit all --file options for standard input (a literal '-' is a file path)",
        value_hint = ValueHint::FilePath
    )]
    files: Vec<PathBuf>,
    #[arg(
        long,
        value_name = "OUTPUT",
        help = "Write the concatenated archive to this path; omit for standard output (a literal '-' is a file path)",
        value_hint = ValueHint::FilePath
    )]
    output: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "no_overwrite",
        help = "Replace an existing --output archive"
    )]
    overwrite: bool,
    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Do not overwrite files. This is the inverse option of --overwrite"
    )]
    no_overwrite: (),
}

impl Command for ConcatCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        concat_entry(self, ctx.umask())
    }
}

#[hooq::hooq(anyhow)]
fn concat_entry(args: ConcatCommand, umask: Umask) -> anyhow::Result<()> {
    ensure!(
        args.output.is_some() || !args.overwrite,
        "--overwrite requires --output PATH for concat"
    );
    for item in &args.files {
        if !utils::fs::is_pna(item)? {
            anyhow::bail!("{} is not a pna file", item.display());
        }
    }
    let destination = resolve_create_destination(args.output, args.overwrite)?;
    destination.open_with(umask, ConcatArchives { files: args.files })
}

/// Copies the raw entries of every input archive (stdin when none) into the opened
/// destination as one archive.
struct ConcatArchives {
    files: Vec<PathBuf>,
}

impl SinkConsumer for ConcatArchives {
    type Output = ();

    fn consume<W: io::Write>(self, writer: W) -> anyhow::Result<()> {
        let mut archive = Archive::write_header(writer)?;
        if self.files.is_empty() {
            let mut input = Archive::read_header(io::stdin().lock())?;
            for entry in input.raw_entries() {
                archive.add_entry(entry?)?;
            }
        } else {
            for item in &self.files {
                append_file_entries(&mut archive, item)?;
            }
        }
        archive.finalize()?;
        Ok(())
    }
}

fn append_file_entries<W: io::Write>(
    output: &mut Archive<W>,
    input: &std::path::Path,
) -> anyhow::Result<()> {
    let archives = collect_split_archives(input)?;
    #[cfg(feature = "memmap")]
    {
        let mmaps = archives
            .into_iter()
            .map(utils::mmap::Mmap::try_from)
            .collect::<io::Result<Vec<_>>>()?;
        let archives = mmaps.iter().map(|m| m.as_ref());
        run_across_archive_bytes(
            archives,
            |reader| {
                for entry in reader.raw_entries_slice() {
                    output.add_entry(entry?)?;
                }
                Ok(())
            },
            false,
        )?;
    }
    #[cfg(not(feature = "memmap"))]
    {
        run_across_archive_readers(
            archives,
            |reader| {
                for entry in reader.raw_entries() {
                    output.add_entry(entry?)?;
                }
                Ok(())
            },
            false,
        )?;
    }
    Ok(())
}
