#[cfg(feature = "memmap")]
use crate::command::core::run_entries;
#[cfg(any(unix, windows))]
use crate::utils::fs::lchown;
use crate::{
    cli::{FileArgsCompat, PasswordArgs},
    command::{
        ask_password,
        core::{
            collect_split_archives, path_lock::PathLocks, read_paths, run_process_archive,
            AclStrategy, KeepOptions, OwnerOptions, PathFilter, PathTransformers,
            PermissionStrategy, TimestampStrategy, XattrStrategy,
        },
        Command,
    },
    utils::{
        self,
        fmt::DurationDisplay,
        fs::{Group, User},
        re::{bsd::SubstitutionRule, gnu::TransformRule},
        GlobPatterns, VCS_FILES,
    },
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::{prelude::*, DataKind, EntryReference, NormalEntry, Permission, ReadOptions};
use std::io::Read;
#[cfg(target_os = "macos")]
use std::os::macos::fs::FileTimesExt;
#[cfg(windows)]
use std::os::windows::fs::FileTimesExt;
use std::{
    borrow::Cow,
    env, fs, io,
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Instant,
};

#[derive(Parser, Clone, Debug)]
#[command(
    group(ArgGroup::new("unstable-include").args(["include"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-from").args(["exclude_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-vcs").args(["exclude_vcs"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(
        ArgGroup::new("from-input")
            .args(["files_from", "exclude_from"])
            .multiple(true)
    ),
    group(ArgGroup::new("null-requires").arg("null").requires("from-input")),
    group(ArgGroup::new("keep-timestamp-flag").args(["keep_timestamp", "no_keep_timestamp"])),
    group(ArgGroup::new("keep-permission-flag").args(["keep_permission", "no_keep_permission"])),
    group(ArgGroup::new("keep-xattr-flag").args(["keep_xattr", "no_keep_xattr"])),
    group(ArgGroup::new("unstable-acl").args(["keep_acl", "no_keep_acl"]).requires("unstable")),
    group(ArgGroup::new("keep-acl-flag").args(["keep_acl", "no_keep_acl"])),
    group(ArgGroup::new("unstable-substitution").args(["substitutions"]).requires("unstable")),
    group(ArgGroup::new("unstable-transform").args(["transforms"]).requires("unstable")),
    group(ArgGroup::new("unstable-keep-old-files").args(["keep_old_files"]).requires("unstable")),
    group(ArgGroup::new("unstable-keep-newer-files").args(["keep_newer_files"]).requires("unstable")),
    group(ArgGroup::new("path-transform").args(["substitutions", "transforms"])),
    group(ArgGroup::new("owner-flag").args(["same_owner", "no_same_owner"])),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
    group(
        ArgGroup::new("overwrite-flag")
            .args(["overwrite", "no_overwrite", "keep_newer_files", "keep_old_files"])
    ),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission", "no_keep_permission"]).requires("unstable")),
))]
pub(crate) struct ExtractCommand {
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[arg(
        long,
        help = "Do not overwrite files. This is the inverse option of --overwrite"
    )]
    no_overwrite: bool,
    #[arg(long, help = "Skip extracting files if a newer version already exists")]
    keep_newer_files: bool,
    #[arg(long, help = "Skip extracting files if they already exist")]
    keep_old_files: bool,
    #[arg(long, help = "Output directory of extracted files", value_hint = ValueHint::DirPath)]
    pub(crate) out_dir: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[arg(
        long,
        visible_alias = "preserve-timestamps",
        help = "Restore the timestamp of the files"
    )]
    pub(crate) keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-timestamps",
        help = "Do not restore timestamp of files. This is the inverse option of --preserve-timestamps"
    )]
    pub(crate) no_keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Restore the permissions of the files (unstable on Windows)"
    )]
    keep_permission: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-permissions",
        help = "Do not restore permissions of files. This is the inverse option of --preserve-permissions"
    )]
    no_keep_permission: bool,
    #[arg(
        long,
        visible_alias = "preserve-xattrs",
        help = "Restore the extended attributes of the files"
    )]
    pub(crate) keep_xattr: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-xattrs",
        help = "Do not restore extended attributes of files. This is the inverse option of --preserve-xattrs"
    )]
    pub(crate) no_keep_xattr: bool,
    #[arg(
        long,
        visible_alias = "preserve-acls",
        help = "Restore the acl of the files (unstable)"
    )]
    pub(crate) keep_acl: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-acls",
        help = "Do not restore acl of files. This is the inverse option of --keep-acl (unstable)"
    )]
    pub(crate) no_keep_acl: bool,
    #[arg(long, help = "Restore user from given name")]
    pub(crate) uname: Option<String>,
    #[arg(long, help = "Restore group from given name")]
    pub(crate) gname: Option<String>,
    #[arg(
        long,
        help = "Overrides the user id in the archive; the user name in the archive will be ignored"
    )]
    pub(crate) uid: Option<u32>,
    #[arg(
        long,
        help = "Overrides the group id in the archive; the group name in the archive will be ignored"
    )]
    pub(crate) gid: Option<u32>,
    #[arg(
        long,
        help = "This is equivalent to --uname \"\" --gname \"\". It causes user and group names in the archive to be ignored in favor of the numeric user and group ids."
    )]
    pub(crate) numeric_owner: bool,
    #[arg(
        long,
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)"
    )]
    include: Option<Vec<String>>,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    exclude: Option<Vec<String>>,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    exclude_from: Option<PathBuf>,
    #[arg(long, help = "Exclude vcs files (unstable)")]
    exclude_vcs: bool,
    #[arg(long, help = "Read extraction patterns from given path (unstable)", value_hint = ValueHint::FilePath)]
    files_from: Option<PathBuf>,
    #[arg(
        long,
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
    #[arg(
        long,
        help = "Remove the specified number of leading path elements. Path names with fewer elements will be silently skipped"
    )]
    strip_components: Option<usize>,
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
        long,
        help = "Try extracting files with the same ownership as exists in the archive"
    )]
    same_owner: bool,
    #[arg(long, help = "Extract files as yourself")]
    no_same_owner: bool,
    #[arg(
        short = 'C',
        long = "cd",
        visible_aliases = ["directory"],
        value_name = "DIRECTORY",
        help = "Change directories after opening the archive but before extracting entries from the archive",
        value_hint = ValueHint::DirPath
    )]
    working_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "chroot() to the current directory after processing any --cd options and before extracting any files"
    )]
    chroot: bool,
    #[arg(
        long,
        help = "Allow extracting symbolic links and hard links that contain root or parent paths"
    )]
    allow_unsafe_links: bool,
    #[command(flatten)]
    pub(crate) file: FileArgsCompat,
}

impl Command for ExtractCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        extract_archive(self)
    }
}
fn extract_archive(args: ExtractCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let start = Instant::now();
    let archive = args.file.archive();
    log::info!("Extract archive {}", archive.display());

    let archives = collect_split_archives(&archive)?;

    let mut exclude = args.exclude.unwrap_or_default();
    if let Some(p) = args.exclude_from {
        exclude.extend(read_paths(p, args.null)?);
    }
    let vcs_patterns = args
        .exclude_vcs
        .then(|| VCS_FILES.iter().copied())
        .into_iter()
        .flatten();
    let filter = PathFilter::new(
        args.include.iter().flatten(),
        exclude.iter().map(|s| s.as_str()).chain(vcs_patterns),
    );

    let mut files = args.file.files();
    if let Some(path) = &args.files_from {
        files.extend(read_paths(path, args.null)?);
    }

    let overwrite_strategy = OverwriteStrategy::from_flags(
        args.overwrite,
        args.no_overwrite,
        args.keep_newer_files,
        args.keep_old_files,
        OverwriteStrategy::Never,
    );
    let keep_options = KeepOptions {
        timestamp_strategy: TimestampStrategy::from_flags(
            args.keep_timestamp,
            args.no_keep_timestamp,
        ),
        permission_strategy: PermissionStrategy::from_flags(
            args.keep_permission,
            args.no_keep_permission,
        ),
        xattr_strategy: XattrStrategy::from_flags(args.keep_xattr, args.no_keep_xattr),
        acl_strategy: AclStrategy::from_flags(args.keep_acl, args.no_keep_acl),
    };
    let owner_options = OwnerOptions::new(
        args.uname,
        args.gname,
        args.uid,
        args.gid,
        args.numeric_owner,
    );
    let output_options = OutputOption {
        overwrite_strategy,
        allow_unsafe_links: args.allow_unsafe_links,
        strip_components: args.strip_components,
        out_dir: args.out_dir,
        filter,
        keep_options,
        owner_options,
        same_owner: !args.no_same_owner,
        path_transformers: PathTransformers::new(args.substitutions, args.transforms),
        path_locks: Arc::new(PathLocks::default()),
        unlink_first: false,
    };
    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
    }
    #[cfg(all(unix, not(target_os = "fuchsia")))]
    if args.chroot {
        std::os::unix::fs::chroot(env::current_dir()?)?;
        env::set_current_dir("/")?;
    }
    #[cfg(not(all(unix, not(target_os = "fuchsia"))))]
    if args.chroot {
        log::warn!("chroot not supported on this platform");
    };
    #[cfg(not(feature = "memmap"))]
    run_extract_archive_reader(
        archives
            .into_iter()
            .map(|it| io::BufReader::with_capacity(64 * 1024, it)),
        files,
        || password.as_deref(),
        output_options,
    )?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    #[cfg(feature = "memmap")]
    run_extract_archive(archives, files, || password.as_deref(), output_options)?;
    log::info!(
        "Successfully extracted an archive in {}",
        DurationDisplay(start.elapsed())
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OverwriteStrategy {
    Never,
    Always,
    KeepNewer,
    KeepOlder,
}

impl OverwriteStrategy {
    pub(crate) const fn from_flags(
        overwrite: bool,
        no_overwrite: bool,
        keep_newer: bool,
        keep_older: bool,
        default_strategy: Self,
    ) -> Self {
        if overwrite {
            Self::Always
        } else if no_overwrite {
            Self::Never
        } else if keep_newer {
            Self::KeepNewer
        } else if keep_older {
            Self::KeepOlder
        } else {
            default_strategy
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OutputOption<'a> {
    pub(crate) overwrite_strategy: OverwriteStrategy,
    pub(crate) allow_unsafe_links: bool,
    pub(crate) strip_components: Option<usize>,
    pub(crate) out_dir: Option<PathBuf>,
    pub(crate) filter: PathFilter<'a>,
    pub(crate) keep_options: KeepOptions,
    pub(crate) owner_options: OwnerOptions,
    pub(crate) same_owner: bool,
    pub(crate) path_transformers: Option<PathTransformers>,
    pub(crate) path_locks: Arc<PathLocks>,
    pub(crate) unlink_first: bool,
}

pub(crate) fn run_extract_archive_reader<'a, 'p, Provider>(
    reader: impl IntoIterator<Item = impl Read> + Send,
    files: Vec<String>,
    mut password_provider: Provider,
    args: OutputOption<'a>,
) -> anyhow::Result<()>
where
    Provider: FnMut() -> Option<&'p [u8]> + Send,
{
    let password = password_provider();
    let patterns = files;
    let mut globs = GlobPatterns::new(patterns.iter().map(|it| it.as_str()))?;

    let mut link_entries = Vec::new();

    let (tx, rx) = std::sync::mpsc::channel();
    rayon::scope_fifo(|s| -> anyhow::Result<()> {
        run_process_archive(reader, password_provider, |entry| {
            let item = entry?;
            let item_path = item.header().path().to_string();
            if !globs.is_empty() && !globs.matches_any(&item_path) {
                log::debug!("Skip: {}", item.header().path());
                return Ok(());
            }
            if matches!(
                item.header().data_kind(),
                DataKind::SymbolicLink | DataKind::HardLink
            ) {
                link_entries.push(item);
                return Ok(());
            }
            let tx = tx.clone();
            let args = args.clone();
            s.spawn_fifo(move |_| {
                tx.send(extract_entry(item, password, &args))
                    .unwrap_or_else(|e| log::error!("{e}: {item_path}"));
            });
            Ok(())
        })?;
        drop(tx);
        Ok(())
    })?;
    for result in rx {
        result?;
    }
    for item in link_entries {
        extract_entry(item, password, &args)?;
    }

    globs.ensure_all_matched()?;
    Ok(())
}

#[cfg(feature = "memmap")]
pub(crate) fn run_extract_archive<'a, 'd, 'p, Provider>(
    archives: impl IntoIterator<Item = &'d [u8]> + Send,
    files: Vec<String>,
    mut password_provider: Provider,
    args: OutputOption<'a>,
) -> anyhow::Result<()>
where
    Provider: FnMut() -> Option<&'p [u8]> + Send,
{
    rayon::scope_fifo(|s| {
        let password = password_provider();
        let mut globs = GlobPatterns::new(files.iter().map(|it| it.as_str()))?;

        let mut link_entries = Vec::<NormalEntry>::new();

        let (tx, rx) = std::sync::mpsc::channel();

        run_entries(archives, password_provider, |entry| {
            let item = entry?;
            let item_path = item.header().path().to_string();
            if !globs.is_empty() && !globs.matches_any(&item_path) {
                log::debug!("Skip: {}", item.header().path());
                return Ok(());
            }
            if matches!(
                item.header().data_kind(),
                DataKind::SymbolicLink | DataKind::HardLink
            ) {
                link_entries.push(item.into());
                return Ok(());
            }
            let tx = tx.clone();
            let args = args.clone();
            s.spawn_fifo(move |_| {
                tx.send(extract_entry(item, password, &args))
                    .unwrap_or_else(|e| log::error!("{e}: {item_path}"));
            });
            Ok(())
        })?;
        drop(tx);
        for result in rx {
            result?;
        }

        for item in link_entries {
            extract_entry(item, password, &args)?;
        }
        globs.ensure_all_matched()?;
        Ok(())
    })
}

pub(crate) fn extract_entry<'a, T>(
    item: NormalEntry<T>,
    password: Option<&[u8]>,
    OutputOption {
        overwrite_strategy,
        allow_unsafe_links,
        strip_components,
        out_dir,
        filter,
        keep_options,
        owner_options,
        same_owner,
        path_transformers,
        path_locks,
        unlink_first,
    }: &OutputOption<'a>,
) -> io::Result<()>
where
    T: AsRef<[u8]>,
    pna::RawChunk<T>: Chunk,
{
    let item_path = item.header().path().as_str();
    if filter.excluded(item_path) {
        return Ok(());
    }
    let item_path = item.header().path().as_path();
    log::debug!("Extract: {}", item_path.display());
    let item_path = if let Some(strip_count) = *strip_components {
        if item_path.components().count() <= strip_count {
            return Ok(());
        }
        Cow::from(PathBuf::from_iter(item_path.components().skip(strip_count)))
    } else {
        Cow::from(item_path)
    };
    let item_path = if let Some(transformers) = path_transformers {
        Cow::from(PathBuf::from(transformers.apply(
            item_path.to_string_lossy(),
            false,
            false,
        )))
    } else {
        item_path
    };
    let path = if let Some(out_dir) = out_dir {
        Cow::from(out_dir.join(item_path))
    } else {
        item_path
    };
    let path = if path.as_os_str().is_empty() {
        Cow::Borrowed(".".as_ref())
    } else {
        path
    };

    let entry_kind = item.header().data_kind();

    let path_lock = path_locks.get(path.as_ref());
    let path_guard = path_lock.lock().expect("path lock mutex poisoned");

    log::debug!("start: {}", path.display());
    let metadata = match fs::symlink_metadata(&path) {
        Ok(meta) => Some(meta),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };
    if let Some(existing) = metadata.as_ref() {
        match overwrite_strategy {
            OverwriteStrategy::Never if !*unlink_first => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} already exists", path.display()),
                ));
            }
            OverwriteStrategy::KeepOlder => {
                log::debug!(
                    "Skipped extracting {}: existing one kept by --keep-older",
                    path.display()
                );
                return Ok(());
            }
            OverwriteStrategy::KeepNewer => {
                if is_existing_newer(existing, &item) {
                    log::debug!(
                        "Skipped extracting {}: newer one already exists (--keep-newer)",
                        path.display()
                    );
                    return Ok(());
                }
            }
            OverwriteStrategy::Always | OverwriteStrategy::Never => (),
        }
    }

    let (had_existing, existing_is_dir) = metadata
        .as_ref()
        .map(|meta| (true, meta.is_dir()))
        .unwrap_or((false, false));
    let unlink_existing =
        *unlink_first && had_existing && (entry_kind != DataKind::Directory || !existing_is_dir);
    let should_overwrite_existing = matches!(
        overwrite_strategy,
        OverwriteStrategy::Always | OverwriteStrategy::KeepNewer
    ) && had_existing;
    if unlink_existing {
        utils::fs::remove_path_all(&path)?;
    }

    if let Some(parent) = path.parent() {
        ensure_directory_components(parent, *unlink_first)?;
    }

    let remove_existing = should_overwrite_existing && !unlink_existing;

    match entry_kind {
        DataKind::File => {
            let mut file = utils::fs::file_create(&path, remove_existing)?;
            if let TimestampStrategy::Always = keep_options.timestamp_strategy {
                let mut times = fs::FileTimes::new();
                if let Some(accessed) = item.metadata().accessed_time() {
                    times = times.set_accessed(accessed);
                }
                if let Some(modified) = item.metadata().modified_time() {
                    times = times.set_modified(modified);
                }
                #[cfg(any(windows, target_os = "macos"))]
                if let Some(created) = item.metadata().created_time() {
                    times = times.set_created(created);
                }
                file.set_times(times)?;
            }
            let mut reader = item.reader(ReadOptions::with_password(password))?;
            io::copy(&mut reader, &mut file)?;
        }
        DataKind::Directory => {
            ensure_directory_components(&path, *unlink_first)?;
        }
        DataKind::SymbolicLink => {
            let reader = item.reader(ReadOptions::with_password(password))?;
            let original = io::read_to_string(reader)?;
            let original = if let Some(substitutions) = path_transformers {
                substitutions.apply(original, true, false)
            } else {
                original
            };
            let original = EntryReference::from_lossy(original);
            if !allow_unsafe_links && is_unsafe_link(&original) {
                log::warn!("Skipped extracting a symbolic link that contains an unsafe link. If you need to extract it, use `--allow-unsafe-links`.");
                return Ok(());
            }
            if remove_existing {
                utils::fs::remove_path_all(&path)?;
            }
            utils::fs::symlink(original, &path)?;
        }
        DataKind::HardLink => {
            let reader = item.reader(ReadOptions::with_password(password))?;
            let original = io::read_to_string(reader)?;
            let original = if let Some(substitutions) = path_transformers {
                substitutions.apply(original, true, false)
            } else {
                original
            };
            let original = EntryReference::from_lossy(original);
            if !allow_unsafe_links && is_unsafe_link(&original) {
                log::warn!("Skipped extracting a hard link that contains an unsafe link. If you need to extract it, use `--allow-unsafe-links`.");
                return Ok(());
            }
            let mut original_path = Cow::from(original.as_path());
            if let Some(strip_count) = *strip_components {
                if original_path.components().count() <= strip_count {
                    log::warn!("Skipped extracting a hard link that pointed at a file which was skipped.: {}", original_path.display());
                    return Ok(());
                }
                original_path = Cow::from(PathBuf::from_iter(
                    original_path.components().skip(strip_count),
                ));
            }
            let original = if let Some(out_dir) = out_dir {
                Cow::from(out_dir.join(original_path))
            } else {
                original_path
            };
            if remove_existing {
                utils::fs::remove_path_all(&path)?;
            }
            fs::hard_link(original, &path)?;
        }
    }
    if let PermissionStrategy::Always = keep_options.permission_strategy {
        if let Some(p) = item.metadata().permission() {
            restore_permissions(*same_owner, &path, p, owner_options)?;
        }
    }
    #[cfg(unix)]
    if let XattrStrategy::Always = keep_options.xattr_strategy {
        match utils::os::unix::fs::xattrs::set_xattrs(&path, item.xattrs()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::Unsupported => {
                log::warn!(
                    "Extended attributes are not supported on filesystem for '{}': {}",
                    path.display(),
                    e
                );
            }
            Err(e) => return Err(e),
        }
    }
    #[cfg(not(unix))]
    if let XattrStrategy::Always = keep_options.xattr_strategy {
        log::warn!("Currently extended attribute is not supported on this platform.");
    }
    #[cfg(feature = "acl")]
    {
        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "macos",
            windows
        ))]
        if let AclStrategy::Always = keep_options.acl_strategy {
            use crate::chunk::{acl_convert_current_platform, AcePlatform, Acl};
            use crate::ext::*;
            use itertools::Itertools;

            let platform = AcePlatform::CURRENT;
            let acls = item.acl()?;
            if let Some((platform, acl)) = acls.into_iter().find_or_first(|(p, _)| p.eq(&platform))
            {
                if !acl.is_empty() {
                    utils::acl::set_facl(
                        &path,
                        acl_convert_current_platform(Acl {
                            platform,
                            entries: acl,
                        }),
                    )?;
                }
            }
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "macos",
            windows
        )))]
        if let AclStrategy::Always = keep_options.acl_strategy {
            log::warn!("Currently acl is not supported on this platform.");
        }
    }
    #[cfg(not(feature = "acl"))]
    if let AclStrategy::Always = keep_options.acl_strategy {
        log::warn!("Please enable `acl` feature and rebuild and install pna.");
    }
    drop(path_guard);
    log::debug!("end: {}", path.display());
    Ok(())
}

#[inline]
fn restore_permissions(
    same_owner: bool,
    path: &Path,
    p: &Permission,
    options: &OwnerOptions,
) -> io::Result<()> {
    let permissions = permissions(p, options);
    #[cfg(unix)]
    if let Some((p, u, g)) = permissions {
        if same_owner {
            match lchown(path, u, g) {
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                    log::warn!("failed to restore owner of {}: {}", path.display(), e)
                }
                r => r?,
            }
        }
        utils::os::unix::fs::chmod(path, p.permissions())?;
    };
    #[cfg(windows)]
    if let Some((p, u, g)) = permissions {
        if same_owner {
            lchown(path, u, g)?;
        }
        utils::os::windows::fs::chmod(path, p.permissions())?;
    }
    #[cfg(not(any(unix, windows)))]
    if let Some(_) = permissions {
        log::warn!("Currently permission is not supported on this platform.");
    }
    Ok(())
}

fn is_existing_newer<T>(metadata: &fs::Metadata, item: &NormalEntry<T>) -> bool
where
    T: AsRef<[u8]>,
{
    if let (Ok(existing_modified), Some(entry_modified)) =
        (metadata.modified(), item.metadata().modified_time())
    {
        existing_modified >= entry_modified
    } else {
        false
    }
}

fn ensure_directory_components(path: &Path, unlink_first: bool) -> io::Result<()> {
    if path.as_os_str().is_empty() {
        return Ok(());
    }
    if !unlink_first {
        return fs::create_dir_all(path);
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => continue,
            Component::ParentDir => {
                current.pop();
                continue;
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                current.push(component.as_os_str());
            }
        }
        if current.as_os_str().is_empty() {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                if meta.is_dir() {
                    continue;
                }
                utils::fs::remove_path_all(&current)?;
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        if let Err(err) = fs::create_dir(&current) {
            if err.kind() != io::ErrorKind::AlreadyExists {
                return Err(err);
            }
        }
    }
    Ok(())
}

fn permissions<'p>(
    permission: &'p Permission,
    owner_options: &'_ OwnerOptions,
) -> Option<(&'p Permission, Option<User>, Option<Group>)> {
    let user = if let Some(uid) = owner_options.uid {
        User::from_uid(uid.into())
    } else {
        search_owner(
            owner_options.uname.as_deref().unwrap_or(permission.uname()),
            permission.uid(),
        )
    };
    let group = if let Some(gid) = owner_options.gid {
        Group::from_gid(gid.into())
    } else {
        search_group(
            owner_options.gname.as_deref().unwrap_or(permission.gname()),
            permission.gid(),
        )
    };
    Some((permission, user.ok(), group.ok()))
}

fn search_owner(name: &str, id: u64) -> io::Result<User> {
    let user = User::from_name(name);
    if user.is_ok() {
        return user;
    }
    User::from_uid((id as u32).into())
}

fn search_group(name: &str, id: u64) -> io::Result<Group> {
    let group = Group::from_name(name);
    if group.is_ok() {
        return group;
    }
    Group::from_gid((id as u32).into())
}

#[inline]
fn is_unsafe_link(reference: &EntryReference) -> bool {
    reference.as_path().components().any(|it| {
        matches!(
            it,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}
