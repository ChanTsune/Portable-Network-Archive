use crate::{
    cli::{
        FileArgs, PasswordArgs, PrivateChunkType, SolidEntriesTransformStrategy,
        SolidEntriesTransformStrategyArgs,
    },
    command::{
        Command, ask_password,
        core::{
            SplitArchiveReader, StagedArchive, TransformStrategyKeepSolid,
            TransformStrategyUnSolid, Umask, collect_split_archives,
        },
    },
    utils::PathPartExt,
};
use clap::{Args, Parser, ValueHint};
use pna::{Metadata, NormalEntry, RawChunk, prelude::*};
use std::path::PathBuf;

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct StripOptions {
    #[arg(
        long = "preserve-timestamps",
        visible_alias = "keep-timestamp",
        help = "Preserve file timestamps"
    )]
    keep_timestamp: bool,
    #[arg(
        long = "preserve-permissions",
        visible_alias = "keep-permission",
        help = "Preserve file permissions"
    )]
    keep_permission: bool,
    #[arg(
        long = "preserve-xattrs",
        visible_alias = "keep-xattr",
        help = "Preserve extended attributes"
    )]
    keep_xattr: bool,
    #[arg(
        long = "preserve-acls",
        visible_alias = "keep-acl",
        help = "Preserve ACLs"
    )]
    keep_acl: bool,
    #[arg(long = "preserve-private-chunks", visible_alias = "keep-private", alias = "preserve-private_chunks", value_name = "CHUNK_TYPE", help = "Preserve private chunks. If no CHUNK_TYPE is specified, all private chunks are preserved", value_delimiter = ',', num_args = 0..)]
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
    pub(crate) file: FileArgs,
}

impl Command for StripCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        strip_metadata(self, ctx.umask())
    }
}

#[hooq::hooq(anyhow)]
fn strip_metadata(args: StripCommand, umask: Umask) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let archive = args.file.archive;
    let mut source = SplitArchiveReader::new(collect_split_archives(&archive)?)?;

    let output_path = args.output.unwrap_or_else(|| archive.remove_part());
    let mut staged = StagedArchive::new(output_path, umask)?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => source.transform_entries(
            staged.as_file_mut(),
            password.as_deref(),
            #[hooq::skip_all]
            |entry| Ok(Some(strip_entry_metadata(entry?, &args.strip_options))),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => source.transform_entries(
            staged.as_file_mut(),
            password.as_deref(),
            #[hooq::skip_all]
            |entry| Ok(Some(strip_entry_metadata(entry?, &args.strip_options))),
            TransformStrategyKeepSolid,
        ),
    }?;

    drop(source);

    staged.commit(None)?;
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
        let own = crate::ext::ResolvedOwnership::from_metadata(entry.metadata());
        metadata =
            metadata
                .with_owner_uid(own.uid.map(pna::OwnerUid::from))
                .with_owner_gid(own.gid.map(pna::OwnerGid::from))
                .with_owner_user_name(
                    own.uname
                        .as_deref()
                        .and_then(crate::command::core::permission::owner_name_opt),
                )
                .with_owner_group_name(
                    own.gname
                        .as_deref()
                        .and_then(crate::command::core::permission::owner_group_name_opt),
                )
                .with_permission_mode(own.mode.map(pna::PermissionMode::from))
                .with_owner_user_sid(own.user_sid.map(|s| {
                    pna::OwnerUserSid::new(s).expect("rescued sid within owner-facet bound")
                }))
                .with_owner_group_sid(own.group_sid.map(|s| {
                    pna::OwnerGroupSid::new(s).expect("rescued sid within owner-facet bound")
                }));
    }
    if options.keep_timestamp {
        metadata = metadata.with_accessed(entry.metadata().accessed());
        metadata = metadata.with_created(entry.metadata().created());
        metadata = metadata.with_modified(entry.metadata().modified());
    }
    if options.keep_xattr {
        metadata = metadata.with_xattrs(entry.metadata().xattrs().to_vec());
    }
    entry = entry.with_metadata(metadata);
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
