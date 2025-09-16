use crate::{
    cli::{PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs},
    command::{
        ask_password,
        commons::{
            collect_split_archives, run_transform_entry, TransformStrategyKeepSolid,
            TransformStrategyUnSolid,
        },
        Command,
    },
    ext::*,
    utils::env::NamedTempFile,
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::{prelude::*, NormalEntry, RawChunk};
use std::{io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(group(ArgGroup::new("archive_arg").args(["file", "archive"]).required(true)))]
pub(crate) struct MigrateCommand {
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
    #[arg(short = 'f', long = "file", value_hint = ValueHint::FilePath)]
    file: Option<PathBuf>,
    #[arg(value_hint = ValueHint::FilePath, hide = true)]
    archive: Option<PathBuf>,
    #[arg(long, help = "Output file path", value_hint = ValueHint::AnyPath)]
    output: PathBuf,
}

impl Command for MigrateCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        migrate_metadata(self)
    }
}

fn migrate_metadata(args: MigrateCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;

    let archive_path = match (args.file, args.archive) {
        (Some(f), _) => f,
        (None, Some(a)) => {
            log::warn!("positional `archive` is deprecated, use `--file` instead");
            a
        }
        _ => unreachable!("required by ArgGroup"),
    };
    let archives = collect_split_archives(&archive_path)?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    let output_path = args.output;
    let mut temp_file =
        NamedTempFile::new(|| output_path.parent().unwrap_or_else(|| ".".as_ref()))?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            |entry| Ok(Some(strip_entry_metadata(entry?)?)),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            |entry| Ok(Some(strip_entry_metadata(entry?)?)),
            TransformStrategyKeepSolid,
        ),
    }?;

    #[cfg(feature = "memmap")]
    drop(mmaps);

    temp_file.persist(output_path)?;
    Ok(())
}

#[inline]
fn strip_entry_metadata<T>(entry: NormalEntry<T>) -> io::Result<NormalEntry<T>>
where
    T: Clone,
    RawChunk<T>: Chunk,
    RawChunk<T>: From<RawChunk>,
{
    let keep_private_chunks = [crate::chunk::faCl, crate::chunk::faCe];
    let acls = entry.acl()?;
    let mut acl = vec![];
    for (platform, entries) in acls {
        acl.push(RawChunk::from_data(crate::chunk::faCl, platform.to_bytes()).into());
        for ace in entries {
            acl.push(RawChunk::from_data(crate::chunk::faCe, ace.to_bytes()).into());
        }
    }

    acl.extend(
        entry
            .extra_chunks()
            .iter()
            .filter(|it| !keep_private_chunks.contains(&it.ty()))
            .cloned(),
    );
    Ok(entry.with_extra_chunks(acl))
}
