use crate::{
    cli::{PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs},
    command::{
        Command, ask_password,
        core::{
            SplitArchiveReader, TransformStrategyKeepSolid, TransformStrategyUnSolid,
            collect_split_archives,
        },
    },
    ext::*,
    utils::env::NamedTempFile,
};
use clap::{Parser, ValueHint};
use pna::{NormalEntry, RawChunk, prelude::*};
use std::{io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct MigrateCommand {
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
    #[arg(short = 'f', long = "file", value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(long, help = "Output file path", value_hint = ValueHint::AnyPath)]
    output: PathBuf,
}

impl Command for MigrateCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        migrate_metadata(self)
    }
}

#[hooq::hooq(anyhow)]
fn migrate_metadata(args: MigrateCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;

    let mut source = SplitArchiveReader::new(collect_split_archives(&args.archive)?)?;

    let output_path = args.output;
    let mut temp_file =
        NamedTempFile::new(|| output_path.parent().unwrap_or_else(|| ".".as_ref()))?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => source.transform_entries(
            temp_file.as_file_mut(),
            password.as_deref(),
            #[hooq::skip_all]
            |entry| Ok(Some(convert_entry_to_owner_facet(entry?)?)),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => source.transform_entries(
            temp_file.as_file_mut(),
            password.as_deref(),
            #[hooq::skip_all]
            |entry| Ok(Some(convert_entry_to_owner_facet(entry?)?)),
            TransformStrategyKeepSolid,
        ),
    }?;

    drop(source);

    temp_file.persist(output_path)?;
    Ok(())
}

#[inline]
fn convert_entry_to_owner_facet<T>(entry: NormalEntry<T>) -> io::Result<NormalEntry<T>>
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

    let own = crate::ext::ResolvedOwnership::from_metadata(entry.metadata());
    let src = entry.metadata();
    let created = src.created();
    let modified = src.modified();
    let accessed = src.accessed();
    let link_target_type = src.link_target_type();
    let new_meta = pna::Metadata::new()
        .with_created(created)
        .with_modified(modified)
        .with_accessed(accessed)
        .with_link_target_type(link_target_type)
        .with_xattrs(src.xattrs().to_vec())
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
        .with_owner_user_sid(
            own.user_sid
                .map(|s| pna::OwnerUserSid::new(s).expect("rescued sid within owner-facet bound")),
        )
        .with_owner_group_sid(
            own.group_sid
                .map(|s| pna::OwnerGroupSid::new(s).expect("rescued sid within owner-facet bound")),
        );
    Ok(entry.with_metadata(new_meta).with_extra_chunks(acl))
}
