use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, FileArgs, HashAlgorithmArgs, PasswordArgs,
    },
    command::{
        ask_password, check_password,
        commons::{
            collect_items, create_entry, entry_option, write_split_archive, CreateOptions,
            KeepOptions, OwnerOptions,
        },
        Command,
    },
    utils,
};
use bytesize::ByteSize;
use clap::{ArgGroup, Parser, ValueHint};
use indicatif::HumanDuration;
use pna::{Archive, SolidEntryBuilder, WriteOptions};
use rayon::ThreadPoolBuilder;
use std::{
    fs::{self, File},
    io::{self, prelude::*},
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["keep_acl"]).requires("unstable")),
    group(ArgGroup::new("unstable-create-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from-stdin").args(["files_from_stdin"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("read-files-from").args(["files_from", "files_from_stdin"])),
    group(ArgGroup::new("store-uname").args(["uname"]).requires("keep_permission")),
    group(ArgGroup::new("store-gname").args(["gname"]).requires("keep_permission")),
    group(ArgGroup::new("store-numeric-owner").args(["numeric_owner"]).requires("keep_permission")),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission"]).requires("unstable")),
))]
pub(crate) struct CreateCommand {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Archiving the directories")]
    pub(crate) keep_dir: bool,
    #[arg(long, help = "Archiving the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Archiving the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Archiving the extended attributes of the files")]
    pub(crate) keep_xattr: bool,
    #[arg(long, help = "Archiving the acl of the files")]
    pub(crate) keep_acl: bool,
    #[arg(long, help = "Split archive by total entry size")]
    pub(crate) split: Option<Option<ByteSize>>,
    #[arg(long, help = "Solid mode archive")]
    pub(crate) solid: bool,
    #[arg(long, help = "Archiving user to the entries from given name")]
    pub(crate) uname: Option<String>,
    #[arg(long, help = "Archiving group to the entries from given name")]
    pub(crate) gname: Option<String>,
    #[arg(
        long,
        help = "Overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id"
    )]
    pub(crate) uid: Option<u32>,
    #[arg(
        long,
        help = "Overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id"
    )]
    pub(crate) gid: Option<u32>,
    #[arg(
        long,
        help = "This is equivalent to --uname \"\" --gname \"\". It causes user and group names to not be stored in the archive"
    )]
    pub(crate) numeric_owner: bool,
    #[arg(long, help = "Read archiving files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) files_from: Option<String>,
    #[arg(long, help = "Read archiving files from stdin (unstable)")]
    pub(crate) files_from_stdin: bool,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) exclude_from: Option<String>,
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) hash: HashAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    pub(crate) exclude: Option<Vec<PathBuf>>,
}

impl Command for CreateCommand {
    fn execute(self) -> io::Result<()> {
        create_archive(self)
    }
}

fn create_archive(args: CreateCommand) -> io::Result<()> {
    let password = ask_password(args.password.clone())?;
    check_password(&password, &args.cipher);
    let start = Instant::now();
    let archive = &args.file.archive;
    if !args.overwrite && archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is already exists", archive.display()),
        ));
    }
    log::info!("Create an archive: {}", archive.display());
    let mut files = args.file.files;
    if args.files_from_stdin {
        files.extend(io::stdin().lines().collect::<io::Result<Vec<_>>>()?);
    } else if let Some(path) = args.files_from {
        files.extend(utils::fs::read_to_lines(path)?);
    }
    let exclude = if args.exclude.is_some() || args.exclude_from.is_some() {
        let mut exclude = Vec::new();
        if let Some(e) = args.exclude {
            exclude.extend(e);
        }
        if let Some(p) = args.exclude_from {
            exclude.extend(utils::fs::read_to_lines(p)?.into_iter().map(PathBuf::from));
        }
        Some(exclude)
    } else {
        None
    };
    let target_items = collect_items(&files, args.recursive, args.keep_dir, exclude)?;

    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
    }
    let max_file_size = args
        .split
        .map(|it| it.unwrap_or(ByteSize::gb(1)).0 as usize);

    let keep_options = KeepOptions {
        keep_timestamp: args.keep_timestamp,
        keep_permission: args.keep_permission,
        keep_xattr: args.keep_xattr,
        keep_acl: args.keep_acl,
    };
    let owner_options = OwnerOptions::new(
        args.uname,
        args.gname,
        args.uid,
        args.gid,
        args.numeric_owner,
    );
    let write_option = entry_option(args.compression, args.cipher, args.hash, password);
    if let Some(size) = max_file_size {
        create_archive_with_split(
            &args.file.archive,
            write_option,
            keep_options,
            owner_options,
            args.solid,
            target_items,
            size,
        )?;
    } else {
        create_archive_file(
            || File::create(&args.file.archive),
            write_option,
            keep_options,
            owner_options,
            args.solid,
            target_items,
        )?;
    }
    log::info!(
        "Successfully created an archive in {}",
        HumanDuration(start.elapsed())
    );
    Ok(())
}

pub(crate) fn create_archive_file<W, F>(
    mut get_writer: F,
    write_option: WriteOptions,
    keep_options: KeepOptions,
    owner_options: OwnerOptions,
    solid: bool,
    target_items: Vec<PathBuf>,
) -> io::Result<()>
where
    W: Write,
    F: FnMut() -> io::Result<W>,
{
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(io::Error::other)?;

    let (tx, rx) = std::sync::mpsc::channel();
    let option = if solid {
        WriteOptions::store()
    } else {
        write_option.clone()
    };
    let create_options = CreateOptions {
        option,
        keep_options,
        owner_options,
    };
    for file in target_items {
        let tx = tx.clone();
        pool.scope_fifo(|s| {
            s.spawn_fifo(|_| {
                log::debug!("Adding: {}", file.display());
                tx.send(create_entry(&file, create_options.clone()))
                    .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
            })
        });
    }

    drop(tx);

    let file = get_writer()?;
    if solid {
        let mut writer = Archive::write_solid_header(file, write_option)?;
        for entry in rx.into_iter() {
            writer.add_entry(entry?)?;
        }
        writer.finalize()?;
    } else {
        let mut writer = Archive::write_header(file)?;
        for entry in rx.into_iter() {
            writer.add_entry(entry?)?;
        }
        writer.finalize()?;
    }
    Ok(())
}

fn create_archive_with_split(
    archive: &Path,
    write_option: WriteOptions,
    keep_options: KeepOptions,
    owner_options: OwnerOptions,
    solid: bool,
    target_items: Vec<PathBuf>,
    max_file_size: usize,
) -> io::Result<()> {
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(io::Error::other)?;

    let (tx, rx) = std::sync::mpsc::channel();
    let option = if solid {
        WriteOptions::store()
    } else {
        write_option.clone()
    };
    let create_options = CreateOptions {
        option,
        keep_options,
        owner_options,
    };
    for file in target_items {
        let tx = tx.clone();
        pool.scope_fifo(|s| {
            s.spawn_fifo(|_| {
                log::debug!("Adding: {}", file.display());
                tx.send(create_entry(&file, create_options.clone()))
                    .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
            })
        });
    }

    drop(tx);

    if solid {
        let mut entries_builder = SolidEntryBuilder::new(write_option)?;
        for entry in rx.into_iter() {
            entries_builder.add_entry(entry?)?;
        }
        let entries = entries_builder.build();
        write_split_archive(archive, [entries].into_iter(), max_file_size)?;
    } else {
        write_split_archive(archive, rx.into_iter(), max_file_size)?;
    }
    Ok(())
}
