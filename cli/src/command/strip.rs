use crate::{
    cli::{
        FileArgsCompat, PasswordArgs, PrivateChunkType, SolidEntriesTransformStrategy,
        SolidEntriesTransformStrategyArgs,
    },
    command::{
        Command, ask_password,
        core::{
            TransformStrategyKeepSolid, TransformStrategyUnSolid, collect_split_archives,
            run_transform_entry,
        },
    },
    utils::{PathPartExt, env::NamedTempFile},
};
use clap::{Args, Parser, ValueHint};
use pna::{Metadata, NormalEntry, RawChunk, prelude::*};
use std::path::PathBuf;

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct StripOptions {
    #[arg(
        long,
        visible_alias = "preserve-timestamps",
        help = "Keep the timestamp of the files"
    )]
    pub(crate) keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Keep the permissions of the files"
    )]
    pub(crate) keep_permission: bool,
    #[arg(
        long,
        visible_alias = "preserve-xattrs",
        help = "Keep the extended attributes of the files"
    )]
    pub(crate) keep_xattr: bool,
    #[arg(
        long,
        visible_alias = "preserve-acls",
        help = "Keep the acl of the files"
    )]
    pub(crate) keep_acl: bool,
    #[arg(long, visible_alias = "preserve-private_chunks", help = "Keep private chunks", value_delimiter = ',', num_args = 0..)]
    pub(crate) keep_private: Option<Vec<PrivateChunkType>>,
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct StripCommand {
    #[command(flatten)]
    pub(crate) strip_options: StripOptions,
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[arg(long, help = "Output file path", value_hint = ValueHint::AnyPath)]
    pub(crate) output: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgsCompat,
}

impl Command for StripCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        strip_metadata(self)
    }
}

fn strip_metadata(args: StripCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let archive = args.file.archive();
    let archives = collect_split_archives(&archive)?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<std::io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    let output_path = args.output.unwrap_or_else(|| archive.remove_part());
    let mut temp_file =
        NamedTempFile::new(|| output_path.parent().unwrap_or_else(|| ".".as_ref()))?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            |entry| Ok(Some(strip_entry_metadata(entry?, &args.strip_options))),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            |entry| Ok(Some(strip_entry_metadata(entry?, &args.strip_options))),
            TransformStrategyKeepSolid,
        ),
    }?;

    #[cfg(feature = "memmap")]
    drop(mmaps);

    temp_file.persist(output_path)?;
    Ok(())
}

#[inline]
fn strip_entry_metadata<T>(mut entry: NormalEntry<T>, options: &StripOptions) -> NormalEntry<T>
where
    T: Clone,
    RawChunk<T>: Chunk,
{
    let mut metadata = Metadata::new();
    if options.keep_permission {
        metadata = metadata.with_permission(entry.metadata().permission().cloned());
    }
    if options.keep_timestamp {
        metadata = metadata.with_accessed(entry.metadata().accessed());
        metadata = metadata.with_created(entry.metadata().created());
        metadata = metadata.with_modified(entry.metadata().modified());
    }
    entry = entry.with_metadata(metadata);
    if !options.keep_xattr {
        entry = entry.with_xattrs(&[]);
    }
    let keep_private_all = options
        .keep_private
        .as_ref()
        .is_some_and(|it| it.is_empty());
    let mut keep_private_chunks = Vec::new();
    if options.keep_acl {
        keep_private_chunks.push(crate::chunk::faCl);
        keep_private_chunks.push(crate::chunk::faCe);
    }
    if let Some(chunks) = &options.keep_private {
        keep_private_chunks.extend(chunks.iter().map(|it| it.0))
    }
    let filtered = entry
        .extra_chunks()
        .iter()
        .filter(|it| keep_private_all || keep_private_chunks.contains(&it.ty()))
        .cloned()
        .collect::<Vec<_>>();
    entry.with_extra_chunks(filtered)
}
