#[cfg(feature = "memmap")]
use crate::command::commons::run_read_entries_mem as run_read_entries;
#[cfg(not(feature = "memmap"))]
use crate::command::commons::run_read_entries_path as run_read_entries;
use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, FileArgs, HashAlgorithmArgs, PasswordArgs,
        SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs,
    },
    command::{
        ask_password, check_password,
        commons::{
            collect_items, create_entry, entry_option, CreateOptions, KeepOptions, OwnerOptions,
            TransformStrategy, TransformStrategyKeepSolid, TransformStrategyUnSolid,
        },
        Command,
    },
    utils::{self, env::temp_dir, PathPartExt},
};
use clap::{ArgGroup, Parser, ValueHint};
use normalize_path::*;
use pna::{Archive, Metadata};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["keep_acl"]).requires("unstable")),
    group(ArgGroup::new("unstable-update-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from-stdin").args(["files_from_stdin"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-from").args(["exclude_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-gitignore").args(["gitignore"]).requires("unstable")),
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
pub(crate) struct UpdateCommand {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
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
    #[arg(
        long,
        help = "Only include files and directories older than the specified date. This compares ctime entries."
    )]
    pub(crate) older_ctime: bool,
    #[arg(
        long,
        help = "Only include files and directories older than the specified date. This compares mtime entries."
    )]
    pub(crate) older_mtime: bool,
    #[arg(
        long,
        help = "Only include files and directories newer than the specified date. This compares ctime entries."
    )]
    pub(crate) newer_ctime: bool,
    #[arg(
        long,
        help = "Only include files and directories newer than the specified date. This compares mtime entries."
    )]
    pub(crate) newer_mtime: bool,
    #[arg(long, help = "Read archiving files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) files_from: Option<String>,
    #[arg(long, help = "Read archiving files from stdin (unstable)")]
    pub(crate) files_from_stdin: bool,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) exclude_from: Option<String>,
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) hash: HashAlgorithmArgs,
    #[command(flatten)]
    pub(crate) transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    pub(crate) exclude: Option<Vec<PathBuf>>,
    #[arg(long, help = "Ignore files from .gitignore (unstable)")]
    pub(crate) gitignore: bool,
    #[arg(long, help = "Follow symbolic links")]
    pub(crate) follow_links: bool,
}

impl Command for UpdateCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        match self.transform_strategy.strategy() {
            SolidEntriesTransformStrategy::UnSolid => {
                update_archive::<TransformStrategyUnSolid>(self)
            }
            SolidEntriesTransformStrategy::KeepSolid => {
                update_archive::<TransformStrategyKeepSolid>(self)
            }
        }
    }
}

fn update_archive<Strategy: TransformStrategy>(args: UpdateCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let archive_path = args.file.archive;
    if !archive_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not exists", archive_path.display()),
        ));
    }
    let option = entry_option(args.compression, args.cipher, args.hash, password.clone());
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
    let create_options = CreateOptions {
        option,
        keep_options,
        owner_options,
    };

    let mut files = args.file.files;
    if args.files_from_stdin {
        files.extend(io::stdin().lines().collect::<io::Result<Vec<_>>>()?);
    } else if let Some(path) = args.files_from {
        files.extend(utils::fs::read_to_lines(path)?);
    }
    let mut exclude = args
        .exclude
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.normalize())
        .collect::<Vec<_>>();
    if let Some(p) = args.exclude_from {
        exclude.extend(
            utils::fs::read_to_lines(p)?
                .into_iter()
                .map(|it| PathBuf::from(it).normalize()),
        );
    }
    let mut target_items = collect_items(
        &files,
        args.recursive,
        args.keep_dir,
        args.gitignore,
        args.follow_links,
        None,
    )?;

    let pool = utils::thread::build()?;

    let (tx, rx) = std::sync::mpsc::channel();

    let random = rand::random::<usize>();
    let temp_dir_path = temp_dir().unwrap_or_else(|| {
        archive_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    });
    fs::create_dir_all(&temp_dir_path)?;
    let outfile_path = temp_dir_path.join(format!("{}.pna.tmp", random));
    let outfile = fs::File::create(&outfile_path)?;
    let mut out_archive = Archive::write_header(outfile)?;

    let need_update_condition = if args.newer_ctime {
        |path: &Path, metadata: &Metadata| -> Option<bool> {
            let meta = fs::metadata(path).ok()?;
            let ctime = meta.created().ok()?;
            let d = metadata.created()?;
            Some(SystemTime::UNIX_EPOCH + d < ctime)
        }
    } else if args.newer_mtime {
        |path: &Path, metadata: &Metadata| -> Option<bool> {
            let meta = fs::metadata(path).ok()?;
            let mtime = meta.modified().ok()?;
            let d = metadata.modified()?;
            Some(SystemTime::UNIX_EPOCH + d < mtime)
        }
    } else if args.older_ctime {
        |path: &Path, metadata: &Metadata| -> Option<bool> {
            let meta = fs::metadata(path).ok()?;
            let ctime = meta.created().ok()?;
            let d = metadata.created()?;
            Some(SystemTime::UNIX_EPOCH + d > ctime)
        }
    } else if args.older_mtime {
        |path: &Path, metadata: &Metadata| -> Option<bool> {
            let meta = fs::metadata(path).ok()?;
            let mtime = meta.modified().ok()?;
            let d = metadata.modified()?;
            Some(SystemTime::UNIX_EPOCH + d > mtime)
        }
    } else {
        |_: &Path, _: &Metadata| -> Option<bool> { Some(true) }
    };

    run_read_entries(&archive_path, |entry| {
        Strategy::transform(&mut out_archive, password.as_deref(), entry, |entry| {
            let entry = entry?;
            let file = entry.header().path().as_path().to_path_buf();
            let normalized_path = file.normalize();
            if target_items.contains(&normalized_path) {
                let entry = if !exclude.contains(&normalized_path)
                    && need_update_condition(&normalized_path, entry.metadata()).unwrap_or(true)
                {
                    let tx = tx.clone();
                    pool.scope_fifo(|s| {
                        s.spawn_fifo(|_| {
                            log::debug!("Updating: {}", file.display());
                            tx.send(create_entry(&file, create_options.clone()))
                                .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
                        });
                    });
                    None
                } else {
                    Some(entry)
                };
                target_items.retain(|p| p.normalize() == normalized_path);
                return Ok(entry);
            }
            Ok(None)
        })
    })?;

    // NOTE: Add new entries
    for file in target_items {
        let tx = tx.clone();
        pool.scope_fifo(|s| {
            s.spawn_fifo(|_| {
                log::debug!("Adding: {}", file.display());
                tx.send(create_entry(&file, create_options.clone()))
                    .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
            });
        });
    }

    drop(tx);
    for entry in rx.into_iter() {
        Strategy::transform(
            &mut out_archive,
            password.as_deref(),
            entry.map(Into::into),
            |entry| entry.map(Some),
        )?;
    }
    out_archive.finalize()?;

    utils::fs::mv(outfile_path, archive_path.remove_part().unwrap())?;

    Ok(())
}
