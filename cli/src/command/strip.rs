use crate::{
    cli::{FileArgs, PasswordArgs, PrivateChunkType},
    command::{ask_password, commons::run_manipulate_entry, Command},
    utils::PathPartExt,
};
use clap::{Args, Parser, ValueHint};
use pna::{prelude::*, Metadata, RawChunk, RegularEntry};
use std::{io, path::PathBuf};

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct StripOptions {
    #[arg(long, help = "Keep the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Keep the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Keep the extended attributes of the files")]
    pub(crate) keep_xattr: bool,
    #[arg(long, help = "Keep the acl of the files")]
    pub(crate) keep_acl: bool,
    #[arg(long, help = "Keep private chunks", value_delimiter = ',', num_args = 0..)]
    pub(crate) keep_private: Option<Vec<PrivateChunkType>>,
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct StripCommand {
    #[command(flatten)]
    pub(crate) strip_options: StripOptions,
    #[arg(long, help = "Output file path", value_hint = ValueHint::AnyPath)]
    pub(crate) output: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
}

impl Command for StripCommand {
    fn execute(self) -> io::Result<()> {
        strip_metadata(self)
    }
}

fn strip_metadata(args: StripCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    run_manipulate_entry(
        args.output
            .unwrap_or_else(|| args.file.archive.remove_part().unwrap()),
        &args.file.archive,
        || password.as_deref(),
        |entry| Ok(strip_entry_metadata(entry?, &args.strip_options)),
    )
}

#[inline]
fn strip_entry_metadata<T>(mut entry: RegularEntry<T>, options: &StripOptions) -> RegularEntry<T>
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
    entry.with_extra_chunks(&filtered)
}
