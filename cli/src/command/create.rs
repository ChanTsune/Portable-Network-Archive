use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, FileArgs, HashAlgorithmArgs, PasswordArgs,
    },
    command::{
        ask_password, check_password,
        commons::{
            collect_items, create_entry, entry_option, write_split_archive, CreateOptions,
            KeepOptions, OwnerOptions, PathTransformers,
        },
        Command,
    },
    utils::{
        self,
        fmt::DurationDisplay,
        re::{bsd::SubstitutionRule, gnu::TransformRule},
    },
};
use bytesize::ByteSize;
use clap::{ArgGroup, Parser, ValueHint};
use pna::{Archive, SolidEntryBuilder, WriteOptions};
use std::{
    fs::{self, File},
    io::{self, prelude::*},
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Parser, Clone, Debug)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["keep_acl"]).requires("unstable")),
    group(ArgGroup::new("unstable-create-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from-stdin").args(["files_from_stdin"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-from").args(["exclude_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-gitignore").args(["gitignore"]).requires("unstable")),
    group(ArgGroup::new("unstable-substitution").args(["substitutions"]).requires("unstable")),
    group(ArgGroup::new("unstable-transform").args(["transforms"]).requires("unstable")),
    group(ArgGroup::new("path-transform").args(["substitutions", "transforms"])),
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
    #[arg(
        long,
        visible_alias = "preserve-timestamps",
        help = "Archiving the timestamp of the files"
    )]
    pub(crate) keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Archiving the permissions of the files"
    )]
    pub(crate) keep_permission: bool,
    #[arg(
        long,
        visible_alias = "preserve-xattrs",
        help = "Archiving the extended attributes of the files"
    )]
    pub(crate) keep_xattr: bool,
    #[arg(
        long,
        visible_alias = "preserve-acls",
        help = "Archiving the acl of the files"
    )]
    pub(crate) keep_acl: bool,
    #[arg(
        long,
        value_name = "size",
        help = "Splits archive by given size in bytes"
    )]
    pub(crate) split: Option<Option<ByteSize>>,
    #[arg(long, help = "Create an archive in solid mode")]
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
    #[arg(long, help = "Ignore files from .gitignore (unstable)")]
    pub(crate) gitignore: bool,
    #[arg(long, help = "Follow symbolic links")]
    pub(crate) follow_links: bool,
    #[arg(
        short = 's',
        value_name = "PATTERN",
        help = "Modify file or archive member names according to pattern that like BSD tar -s option"
    )]
    substitutions: Option<Vec<SubstitutionRule>>,
    #[arg(
        long = "transform",
        visible_alias = "xform",
        value_name = "PATTERN",
        help = "Modify file or archive member names according to pattern that like GNU tar -transform option"
    )]
    transforms: Option<Vec<TransformRule>>,
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
    #[arg(long, help = "Windows-only: Store file attributes (ReadOnly, Hidden, System, etc.) as an xattr named 'windows.file_attributes'.")]
    pub(crate) store_windows_attributes: bool,
    #[arg(long, help = "Windows-only: Store common file properties (Title, Subject, Author, Keywords, Comment) as xattrs.")]
    pub(crate) store_windows_properties: bool,
}

impl Command for CreateCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        create_archive(self)
    }
}

fn create_archive(args: CreateCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
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
    let exclude = {
        let mut exclude = Vec::new();
        if let Some(e) = args.exclude {
            exclude.extend(e);
        }
        if let Some(p) = args.exclude_from {
            exclude.extend(utils::fs::read_to_lines(p)?.into_iter().map(PathBuf::from));
        }
        exclude
    };
    let target_items = collect_items(
        &files,
        args.recursive,
        args.keep_dir,
        args.gitignore,
        args.follow_links,
        exclude,
    )?;

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
        restore_windows_attributes: false, // Not used in create
        store_windows_attributes: args.store_windows_attributes,
        store_windows_properties: args.store_windows_properties,
        restore_windows_properties: false, // Not used in create
    };
    let owner_options = OwnerOptions::new(
        args.uname,
        args.gname,
        args.uid,
        args.gid,
        args.numeric_owner,
    );
    let path_transformers = PathTransformers::new(args.substitutions, args.transforms);
    let password = password.as_deref();
    let write_option = entry_option(args.compression, args.cipher, args.hash, password);
    if let Some(size) = max_file_size {
        create_archive_with_split(
            &args.file.archive,
            write_option,
            keep_options,
            owner_options,
            args.solid,
            path_transformers,
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
            path_transformers,
            target_items,
        )?;
    }
    log::info!(
        "Successfully created an archive in {}",
        DurationDisplay(start.elapsed())
    );
    Ok(())
}

pub(crate) fn create_archive_file<W, F>(
    mut get_writer: F,
    write_option: WriteOptions,
    keep_options: KeepOptions,
    owner_options: OwnerOptions,
    solid: bool,
    path_transformers: Option<PathTransformers>,
    target_items: Vec<PathBuf>,
) -> io::Result<()>
where
    W: Write,
    F: FnMut() -> io::Result<W>,
{
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
        rayon::scope_fifo(|s| {
            s.spawn_fifo(|_| {
                log::debug!("Adding: {}", file.display());
                tx.send(create_entry(&file, &create_options, &path_transformers))
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
    path_transformers: Option<PathTransformers>,
    target_items: Vec<PathBuf>,
    max_file_size: usize,
) -> io::Result<()> {
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
        rayon::scope_fifo(|s| {
            s.spawn_fifo(|_| {
                log::debug!("Adding: {}", file.display());
                tx.send(create_entry(&file, &create_options, &path_transformers))
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
