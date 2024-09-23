use crate::{
    cli::{FileArgs, PasswordArgs, PrivateChunkType},
    command::{ask_password, commons::run_entries, Command},
    utils::{self, PathPartExt},
};
use clap::{Args, Parser, ValueHint};
use pna::{prelude::*, Archive, Metadata, RawChunk, RegularEntry};
use std::{env::temp_dir, fs, io, path::PathBuf};

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
    let outfile_path = if let Some(output) = &args.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        output.clone()
    } else {
        let random = rand::random::<usize>();
        temp_dir().join(format!("{}.pna.tmp", random))
    };
    let outfile = fs::File::create(&outfile_path)?;
    let mut out_archive = Archive::write_header(outfile)?;

    run_entries(
        &args.file.archive,
        || password.as_deref(),
        |entry| {
            let entry = strip_entry_metadata(entry?, &args.strip_options);
            out_archive.add_entry(entry)?;
            Ok(())
        },
    )?;
    out_archive.finalize()?;

    if args.output.is_none() {
        utils::fs::mv(outfile_path, args.file.archive.remove_part().unwrap())?;
    }
    Ok(())
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
