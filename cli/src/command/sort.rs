use crate::{
    cli::PasswordArgs,
    command::{
        ask_password,
        commons::{collect_split_archives, run_entries},
        Command,
    },
    utils::{env::NamedTempFile, PathPartExt},
};
use clap::{Parser, ValueEnum, ValueHint};
use pna::{Archive, NormalEntry};
use std::path::PathBuf;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, ValueEnum)]
pub(crate) enum SortBy {
    Name,
    Ctime,
    Mtime,
    Atime,
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SortCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(long, help = "Output file path", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    #[arg(long = "by", value_enum, num_args = 1.., default_values_t = [SortBy::Name])]
    by: Vec<SortBy>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SortCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        sort_archive(self)
    }
}

fn sort_archive(args: SortCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let archives = collect_split_archives(&args.archive)?;
    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<std::io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());
    let mut entries = Vec::<NormalEntry<_>>::new();
    run_entries(
        archives,
        || password.as_deref(),
        |entry| {
            entries.push(entry?);
            Ok(())
        },
    )?;

    entries.sort_by(|a, b| {
        for by in &args.by {
            let ord = match by {
                SortBy::Name => a.header().path().cmp(b.header().path()),
                SortBy::Ctime => a.metadata().created().cmp(&b.metadata().created()),
                SortBy::Mtime => a.metadata().modified().cmp(&b.metadata().modified()),
                SortBy::Atime => a.metadata().accessed().cmp(&b.metadata().accessed()),
            };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });

    let mut temp_file =
        NamedTempFile::new(|| args.archive.parent().unwrap_or_else(|| ".".as_ref()))?;
    let mut archive = Archive::write_header(temp_file.as_file_mut())?;
    for entry in entries {
        archive.add_entry(entry)?;
    }
    archive.finalize()?;

    #[cfg(feature = "memmap")]
    drop(mmaps);

    let output = args
        .output
        .unwrap_or_else(|| args.archive.remove_part().unwrap());
    temp_file.persist(output)?;

    Ok(())
}
