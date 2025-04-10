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
};
use clap::{Parser, ValueHint};
use pna::{prelude::*, NormalEntry, RawChunk};
use std::{io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct MigrateCommand {
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(long, help = "Output file path", value_hint = ValueHint::AnyPath)]
    output: PathBuf,
}

impl Command for MigrateCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        migrate_metadata(self)
    }
}

fn migrate_metadata(args: MigrateCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;

    let archives = collect_split_archives(&args.archive)?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            args.output,
            archives,
            || password.as_deref(),
            |entry| Ok(Some(strip_entry_metadata(entry?)?)),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            args.output,
            archives,
            || password.as_deref(),
            |entry| Ok(Some(strip_entry_metadata(entry?)?)),
            TransformStrategyKeepSolid,
        ),
    }
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
