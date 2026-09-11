use crate::{
    cli::{
        ArchiveFileArgs, ArchiveOutputArgs, FileOperands, PasswordArgs, PrivateChunkType,
        SolidEntriesTransformStrategyArgs,
    },
    command::{
        Command, ask_password,
        core::{
            Umask,
            archive_destination::resolve_transform_destination,
            rewrite::{EntryTransform, execute_archive_transform},
        },
    },
    utils::GlobPatterns,
};
use clap::{Args, Parser};
use pna::{Metadata, NormalEntry, RawChunk, prelude::*};
use std::{borrow::Cow, io};

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
    #[command(flatten)]
    output: ArchiveOutputArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) archive: ArchiveFileArgs,
    #[command(flatten)]
    pub(crate) files: FileOperands,
}

impl Command for StripCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        strip_metadata(self, ctx.umask())
    }
}

#[hooq::hooq(anyhow)]
fn strip_metadata(args: StripCommand, umask: Umask) -> anyhow::Result<()> {
    let source = args.archive.source();
    let destination =
        resolve_transform_destination(&source, args.output.output, args.output.overwrite)?;
    let password = ask_password(args.password)?;
    let globs = if args.files.files.is_empty() {
        None
    } else {
        Some(GlobPatterns::new(
            args.files.files.iter().map(String::as_str),
        )?)
    };
    execute_archive_transform(
        source,
        destination,
        umask,
        password.as_deref(),
        args.transform_strategy.strategy(),
        StripTransform {
            options: args.strip_options,
            globs,
        },
    )
}

struct StripTransform<'g> {
    options: StripOptions,
    /// `None` selects every entry.
    globs: Option<GlobPatterns<'g>>,
}

impl EntryTransform for StripTransform<'_> {
    fn transform<'d>(
        &mut self,
        entry: NormalEntry<Cow<'d, [u8]>>,
    ) -> io::Result<Option<NormalEntry<Cow<'d, [u8]>>>> {
        let selected = match &mut self.globs {
            Some(globs) => globs.matches_any(entry.name()),
            None => true,
        };
        Ok(Some(if selected {
            strip_entry_metadata(entry, &self.options)
        } else {
            entry
        }))
    }

    fn patterns(&self) -> Option<&GlobPatterns<'_>> {
        self.globs.as_ref()
    }
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
