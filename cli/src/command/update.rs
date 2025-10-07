#[cfg(not(feature = "memmap"))]
use crate::command::core::run_read_entries;
#[cfg(feature = "memmap")]
use crate::command::core::run_read_entries_mem as run_read_entries;
use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, DateTime, FileArgs, HashAlgorithmArgs,
        PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs,
    },
    command::{
        ask_password, check_password,
        core::{
            collect_items, collect_split_archives, create_entry, entry_option, read_paths,
            read_paths_stdin, CreateOptions, KeepOptions, OwnerOptions, PathFilter,
            PathTransformers, TimeOptions, TransformStrategy, TransformStrategyKeepSolid,
            TransformStrategyUnSolid,
        },
        Command,
    },
    utils::{
        env::NamedTempFile,
        re::{bsd::SubstitutionRule, gnu::TransformRule},
        PathPartExt, VCS_FILES,
    },
};
use clap::{ArgGroup, Parser, ValueHint};
use indexmap::IndexMap;
use pna::{Archive, EntryName, Metadata};
use std::{env, fs, io, path::PathBuf, time::SystemTime};

#[derive(Parser, Clone, Debug)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["keep_acl"]).requires("unstable")),
    group(ArgGroup::new("unstable-include").args(["include"]).requires("unstable")),
    group(ArgGroup::new("unstable-update-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from-stdin").args(["files_from_stdin"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-from").args(["exclude_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-gitignore").args(["gitignore"]).requires("unstable")),
    group(ArgGroup::new("unstable-substitution").args(["substitutions"]).requires("unstable")),
    group(ArgGroup::new("unstable-transform").args(["transforms"]).requires("unstable")),
    group(ArgGroup::new("path-transform").args(["substitutions", "transforms"])),
    group(ArgGroup::new("read-files-from").args(["files_from", "files_from_stdin"])),
    group(
        ArgGroup::new("from-input")
            .args(["files_from", "files_from_stdin", "exclude_from"])
            .multiple(true)
    ),
    group(ArgGroup::new("null-requires").arg("null").requires("from-input")),
    group(ArgGroup::new("store-uname").args(["uname"]).requires("keep_permission")),
    group(ArgGroup::new("store-gname").args(["gname"]).requires("keep_permission")),
    group(ArgGroup::new("store-numeric-owner").args(["numeric_owner"]).requires("keep_permission")),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
    group(ArgGroup::new("recursive-flag").args(["recursive", "no_recursive"])),
    group(ArgGroup::new("keep-dir-flag").args(["keep_dir", "no_keep_dir"])),
    group(ArgGroup::new("mtime-flag").args(["clamp_mtime"]).requires("mtime")),
    group(ArgGroup::new("atime-flag").args(["clamp_atime"]).requires("atime")),
    group(ArgGroup::new("unstable-exclude-vcs").args(["exclude_vcs"]).requires("unstable")),
    group(ArgGroup::new("unstable-follow_command_links").args(["follow_command_links"]).requires("unstable")),
    group(ArgGroup::new("unstable-one-file-system").args(["one_file_system"]).requires("unstable")),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission"]).requires("unstable")),
))]
pub(crate) struct UpdateCommand {
    #[arg(
        long,
        help = "Stay in the same file system when collecting files (unstable)"
    )]
    one_file_system: bool,
    #[arg(
        short,
        long,
        visible_alias = "recursion",
        help = "Add the directory to the archive recursively",
        default_value_t = true
    )]
    recursive: bool,
    #[arg(
        long,
        visible_alias = "no-recursion",
        help = "Do not recursively add directories to the archives. This is the inverse option of --recursive"
    )]
    no_recursive: bool,
    #[arg(long, help = "Archiving the directories")]
    keep_dir: bool,
    #[arg(
        long,
        help = "Do not archive directories. This is the inverse option of --keep-dir"
    )]
    no_keep_dir: bool,
    #[arg(
        long,
        visible_alias = "preserve-timestamps",
        help = "Archiving the timestamp of the files"
    )]
    pub(crate) keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Archiving the permissions of the files (unstable on Windows)"
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
        help = "Archiving the acl of the files (unstable)"
    )]
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
    #[arg(long, help = "Overrides the creation time read from disk")]
    ctime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the creation time of the entries to the specified time by --ctime"
    )]
    clamp_ctime: bool,
    #[arg(long, help = "Overrides the access time read from disk")]
    atime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the access time of the entries to the specified time by --atime"
    )]
    clamp_atime: bool,
    #[arg(long, help = "Overrides the modification time read from disk")]
    mtime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the modification time of the entries to the specified time by --mtime"
    )]
    clamp_mtime: bool,
    #[arg(
        long,
        help = "Only include files and directories older than the specified date. This compares ctime entries."
    )]
    older_ctime: Option<DateTime>,
    #[arg(
        long,
        help = "Only include files and directories older than the specified date. This compares mtime entries."
    )]
    older_mtime: Option<DateTime>,
    #[arg(
        long,
        help = "Only include files and directories newer than the specified date. This compares ctime entries."
    )]
    newer_ctime: Option<DateTime>,
    #[arg(
        long,
        help = "Only include files and directories newer than the specified date. This compares mtime entries."
    )]
    newer_mtime: Option<DateTime>,
    #[arg(long, help = "Read archiving files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) files_from: Option<String>,
    #[arg(long, help = "Read archiving files from stdin (unstable)")]
    pub(crate) files_from_stdin: bool,
    #[arg(
        long,
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)"
    )]
    include: Option<Vec<String>>,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    exclude: Option<Vec<String>>,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    exclude_from: Option<String>,
    #[arg(long, help = "Exclude vcs files (unstable)")]
    exclude_vcs: bool,
    #[arg(
        short = 's',
        value_name = "PATTERN",
        help = "Modify file or archive member names according to pattern that like BSD tar -s option (unstable)"
    )]
    substitutions: Option<Vec<SubstitutionRule>>,
    #[arg(
        long = "transform",
        visible_alias = "xform",
        value_name = "PATTERN",
        help = "Modify file or archive member names according to pattern that like GNU tar -transform option (unstable)"
    )]
    transforms: Option<Vec<TransformRule>>,
    #[arg(
        short = 'C',
        long = "cd",
        visible_aliases = ["directory"],
        value_name = "DIRECTORY",
        help = "changes the directory before adding the following files",
        value_hint = ValueHint::DirPath
    )]
    working_dir: Option<PathBuf>,
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
    #[arg(
        long,
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
    #[arg(long, help = "Ignore files from .gitignore (unstable)")]
    pub(crate) gitignore: bool,
    #[arg(long, visible_aliases = ["dereference"], help = "Follow symbolic links")]
    follow_links: bool,
    #[arg(
        short = 'H',
        long,
        help = "Follow symbolic links named on the command line"
    )]
    follow_command_links: bool,
}

impl Command for UpdateCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
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

fn update_archive<Strategy: TransformStrategy>(args: UpdateCommand) -> anyhow::Result<()> {
    let current_dir = env::current_dir()?;
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let archive_path = &args.file.archive;
    if !archive_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not exists", archive_path.display()),
        )
        .into());
    }
    let password = password.as_deref();
    let option = entry_option(args.compression, args.cipher, args.hash, password);
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
    let time_options = TimeOptions {
        mtime: args.mtime.map(|it| it.to_system_time()),
        clamp_mtime: args.clamp_mtime,
        ctime: args.ctime.map(|it| it.to_system_time()),
        clamp_ctime: args.clamp_ctime,
        atime: args.atime.map(|it| it.to_system_time()),
        clamp_atime: args.clamp_atime,
    };
    let time_filters = TimeFilters {
        ctime: TimeFilter {
            newer_than: args.newer_ctime.map(|it| it.to_system_time()),
            older_than: args.older_ctime.map(|it| it.to_system_time()),
        },
        mtime: TimeFilter {
            newer_than: args.newer_mtime.map(|it| it.to_system_time()),
            older_than: args.older_mtime.map(|it| it.to_system_time()),
        },
    };
    let create_options = CreateOptions {
        option,
        keep_options,
        owner_options,
        time_options,
    };
    let path_transformers = PathTransformers::new(args.substitutions, args.transforms);

    let archives = collect_split_archives(&args.file.archive)?;

    let mut files = args.file.files;
    if args.files_from_stdin {
        files.extend(read_paths_stdin(args.null)?);
    } else if let Some(path) = args.files_from {
        files.extend(read_paths(path, args.null)?);
    }
    let filter = {
        let mut exclude = args.exclude.unwrap_or_default();
        if let Some(p) = args.exclude_from {
            exclude.extend(read_paths(p, args.null)?);
        }
        if args.exclude_vcs {
            exclude.extend(VCS_FILES.iter().map(|it| String::from(*it)))
        }
        PathFilter {
            include: args.include.unwrap_or_default().into(),
            exclude: exclude.into(),
        }
    };

    let archive_path = current_dir.join(args.file.archive);
    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
    }

    let target_items = collect_items(
        &files,
        !args.no_recursive,
        args.keep_dir,
        args.gitignore,
        args.follow_links,
        args.follow_command_links,
        args.one_file_system,
        &filter,
    )?;

    let (tx, rx) = std::sync::mpsc::channel();

    let mut temp_file =
        NamedTempFile::new(|| archive_path.parent().unwrap_or_else(|| ".".as_ref()))?;
    let mut out_archive = Archive::write_header(temp_file.as_file_mut())?;

    let mut target_files_mapping = target_items
        .into_iter()
        .map(|(it, store)| (EntryName::from_lossy(&it), (it, store)))
        .collect::<IndexMap<_, _>>();

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    rayon::scope_fifo(|s| -> anyhow::Result<()> {
        run_read_entries(archives, |entry| {
            Strategy::transform(&mut out_archive, password, entry, |entry| {
                let entry = entry?;
                if let Some((target_path, store)) =
                    target_files_mapping.swap_remove(entry.header().path())
                {
                    let fs_meta = fs::symlink_metadata(&target_path)?;
                    let need_update = is_newer_than_archive(&fs_meta, entry.metadata())
                        .unwrap_or(true)
                        && time_filters.is_retain(&fs_meta);
                    if need_update {
                        let tx = tx.clone();
                        let create_options = create_options.clone();
                        let path_transformers = path_transformers.clone();
                        s.spawn_fifo(move |_| {
                            log::debug!("Updating: {}", target_path.display());
                            let target_path = (target_path, store);
                            tx.send(create_entry(
                                &target_path,
                                &create_options,
                                &path_transformers,
                            ))
                            .unwrap_or_else(|e| log::error!("{e}: {}", target_path.0.display()));
                        });
                        Ok(None)
                    } else {
                        Ok(Some(entry))
                    }
                } else {
                    Ok(Some(entry))
                }
            })
        })?;

        // NOTE: Add new entries
        for (_, (file, store)) in target_files_mapping {
            let tx = tx.clone();
            let create_options = create_options.clone();
            let path_transformers = path_transformers.clone();
            s.spawn_fifo(move |_| {
                log::debug!("Adding: {}", file.display());
                let file = (file, store);
                tx.send(create_entry(&file, &create_options, &path_transformers))
                    .unwrap_or_else(|e| log::error!("{e}: {}", file.0.display()));
            });
        }
        drop(tx);
        Ok(())
    })?;

    for entry in rx.into_iter() {
        Strategy::transform(&mut out_archive, password, entry.map(Into::into), |entry| {
            entry.map(Some)
        })?;
    }
    out_archive.finalize()?;

    #[cfg(feature = "memmap")]
    drop(mmaps);

    temp_file.persist(archive_path.remove_part().unwrap())?;

    Ok(())
}

fn is_newer_than_archive(fs_meta: &fs::Metadata, metadata: &Metadata) -> Option<bool> {
    let mtime = fs_meta.modified().ok()?;
    let d = metadata.modified()?;
    Some(SystemTime::UNIX_EPOCH + d < mtime)
}

pub(crate) struct TimeFilter {
    pub(crate) newer_than: Option<SystemTime>,
    pub(crate) older_than: Option<SystemTime>,
}

impl TimeFilter {
    fn is_retain(&self, time: Option<SystemTime>) -> bool {
        if let Some(newer) = self.newer_than {
            if let Some(t) = time {
                if t < newer {
                    return false;
                }
            }
        }
        if let Some(older) = self.older_than {
            if let Some(t) = time {
                if t > older {
                    return false;
                }
            }
        }
        true
    }
}

pub(crate) struct TimeFilters {
    pub(crate) ctime: TimeFilter,
    pub(crate) mtime: TimeFilter,
}

impl TimeFilters {
    #[inline]
    pub(crate) fn is_retain(&self, fs_meta: &fs::Metadata) -> bool {
        self.is_retain_t(fs_meta.created().ok(), fs_meta.modified().ok())
    }

    fn is_retain_t(&self, fs_ctime: Option<SystemTime>, fs_mtime: Option<SystemTime>) -> bool {
        self.ctime.is_retain(fs_ctime) && self.mtime.is_retain(fs_mtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(3600)
    }

    fn past() -> SystemTime {
        SystemTime::UNIX_EPOCH
    }

    fn future() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(7200)
    }

    #[test]
    fn test_is_retain_t_no_filters() {
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));
        assert!(filters.is_retain_t(None, None));
    }

    #[test]
    fn test_is_retain_t_newer_ctime() {
        // Case 1: newer_ctime is set, fs_ctime is older -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(!filters.is_retain_t(Some(past()), Some(now())));

        // Case 2: newer_ctime is set, fs_ctime is newer -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));

        // Case 3: newer_ctime is set, fs_ctime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(None, Some(now())));
    }

    #[test]
    fn test_is_retain_t_older_ctime() {
        // Case 1: older_ctime is set, fs_ctime is newer -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(!filters.is_retain_t(Some(future()), Some(now())));

        // Case 2: older_ctime is set, fs_ctime is older -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));

        // Case 3: older_ctime is set, fs_ctime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(None, Some(now())));
    }

    #[test]
    fn test_is_retain_t_newer_mtime() {
        // Case 1: newer_mtime is set, fs_mtime is older -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
        };
        assert!(!filters.is_retain_t(Some(now()), Some(past())));

        // Case 2: newer_mtime is set, fs_mtime is newer -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));

        // Case 3: newer_mtime is set, fs_mtime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), None));
    }

    #[test]
    fn test_is_retain_t_older_mtime() {
        // Case 1: older_mtime is set, fs_mtime is newer -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
        };
        assert!(!filters.is_retain_t(Some(now()), Some(future())));

        // Case 2: older_mtime is set, fs_mtime is older -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(future()),
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));

        // Case 3: older_mtime is set, fs_mtime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
        };
        assert!(filters.is_retain_t(Some(now()), None));
    }

    #[test]
    fn test_is_retain_t_all_filters_retain() {
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));
    }

    #[test]
    fn test_is_retain_t_all_filters_not_retain_ctime() {
        // newer_ctime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
        };
        assert!(!filters.is_retain_t(Some(past()), Some(now())));

        // older_ctime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(now()),
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
        };
        assert!(!filters.is_retain_t(Some(future()), Some(now())));
    }

    #[test]
    fn test_is_retain_t_all_filters_not_retain_mtime() {
        // newer_mtime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: Some(future()),
            },
        };
        assert!(!filters.is_retain_t(Some(now()), Some(past())));

        // older_mtime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(now()),
            },
        };
        assert!(!filters.is_retain_t(Some(now()), Some(future())));
    }

    #[test]
    fn test_is_retain_t_mixed_filters_and_none_fs_times() {
        // newer_ctime set, fs_ctime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(None, Some(now())));

        // older_ctime set, fs_ctime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(None, Some(now())));

        // newer_mtime set, fs_mtime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), None));

        // older_mtime set, fs_mtime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
        };
        assert!(filters.is_retain_t(Some(now()), None));
    }
}
