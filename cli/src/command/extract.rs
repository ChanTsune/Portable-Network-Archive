#[cfg(feature = "memmap")]
use crate::command::core::run_entries;
#[cfg(feature = "acl")]
use crate::ext::*;
#[cfg(any(unix, windows))]
use crate::utils::fs::lchown;
use crate::{
    cli::{FileArgsCompat, PasswordArgs},
    command::{
        Command, ask_password,
        core::{
            AclStrategy, KeepOptions, OwnerOptions, PathFilter, PathTransformers, PathnameEditor,
            PermissionStrategy, TimestampStrategy, XattrStrategy, apply_chroot,
            collect_split_archives,
            path_lock::PathLocks,
            re::{bsd::SubstitutionRule, gnu::TransformRule},
            read_paths, run_process_archive,
        },
    },
    utils::{
        self, GlobPatterns, PathWithCwd, VCS_FILES,
        fmt::DurationDisplay,
        fs::{Group, User},
    },
};
use anyhow::Context;
use clap::{ArgGroup, Parser, ValueHint};
use pna::{DataKind, EntryReference, NormalEntry, Permission, ReadOptions, prelude::*};
#[cfg(target_os = "macos")]
use std::os::macos::fs::FileTimesExt;
#[cfg(windows)]
use std::os::windows::fs::FileTimesExt;
use std::{
    borrow::Cow,
    env, fs,
    io::{self, prelude::*},
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Instant,
};

#[derive(Parser, Clone, Debug)]
#[command(
    group(
        ArgGroup::new("from-input")
            .args(["files_from", "exclude_from"])
            .multiple(true)
    ),
    group(ArgGroup::new("null-requires").arg("null").requires("from-input")),
    group(ArgGroup::new("keep-timestamp-flag").args(["keep_timestamp", "no_keep_timestamp"])),
    group(ArgGroup::new("keep-permission-flag").args(["keep_permission", "no_keep_permission"])),
    group(ArgGroup::new("keep-xattr-flag").args(["keep_xattr", "no_keep_xattr"])),
    group(ArgGroup::new("keep-acl-flag").args(["keep_acl", "no_keep_acl"])),
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
    #[arg(
        long,
        requires = "unstable",
        help = "Skip extracting files if a newer version already exists (unstable)"
    )]
    keep_newer_files: bool,
    #[arg(
        long,
        requires = "unstable",
        help = "Skip extracting files if they already exist (unstable)"
    )]
    keep_old_files: bool,
    #[arg(long, value_name = "DIRECTORY", help = "Output directory of extracted files", value_hint = ValueHint::DirPath)]
    out_dir: Option<PathBuf>,
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
        requires = "unstable",
        help = "Restore ACLs (unstable)"
    )]
    keep_acl: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-acls",
        requires = "unstable",
        help = "Do not restore ACLs. This is the inverse option of --keep-acl (unstable)"
    )]
    no_keep_acl: bool,
    #[arg(long, value_name = "NAME", help = "Restore user from given name")]
    uname: Option<String>,
    #[arg(long, value_name = "NAME", help = "Restore group from given name")]
    gname: Option<String>,
    #[arg(
        long,
        value_name = "ID",
        help = "Overrides the user id in the archive; the user name in the archive will be ignored"
    )]
    uid: Option<u32>,
    #[arg(
        long,
        value_name = "ID",
        help = "Overrides the group id in the archive; the group name in the archive will be ignored"
    )]
    gid: Option<u32>,
    #[arg(
        long,
        help = "This is equivalent to --uname \"\" --gname \"\". It causes user and group names in the archive to be ignored in favor of the numeric user and group ids."
    )]
    numeric_owner: bool,
    #[arg(
        long,
        value_name = "PATTERN",
        requires = "unstable",
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)"
    )]
    include: Option<Vec<String>>,
    #[arg(
        long,
        value_name = "PATTERN",
        requires = "unstable",
        help = "Exclude path glob (unstable)",
        value_hint = ValueHint::AnyPath
    )]
    exclude: Option<Vec<String>>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help = "Read exclude files from given path (unstable)",
        value_hint = ValueHint::FilePath
    )]
    exclude_from: Option<PathBuf>,
    #[arg(
        long,
        requires = "unstable",
        help = "Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`) (unstable)"
    )]
    exclude_vcs: bool,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help = "Read extraction patterns from given path (unstable)",
        value_hint = ValueHint::FilePath
    )]
    files_from: Option<PathBuf>,
    #[arg(
        long,
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
    #[arg(
        long,
        value_name = "N",
        help = "Remove the specified number of leading path elements. Path names with fewer elements will be silently skipped"
    )]
    strip_components: Option<usize>,
    #[arg(
        short = 's',
        value_name = "PATTERN",
        requires = "unstable",
        help = "Modify file or archive member names according to pattern that like BSD tar -s option (unstable)"
    )]
    substitutions: Option<Vec<SubstitutionRule>>,
    #[arg(
        long = "transform",
        visible_alias = "xform",
        value_name = "PATTERN",
        requires = "unstable",
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
        help = "chroot() to the current directory after processing any --cd options and before extracting any files (requires root privileges)"
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
    fn execute(self, _ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        extract_archive(self)
    }
}
fn extract_archive(args: ExtractCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password).with_context(|| "reading password")?;
    let start = Instant::now();
    let archive = args.file.archive();
    log::info!("Extract archive {}", PathWithCwd::new(&archive));

    let archives = collect_split_archives(&archive)
        .with_context(|| format!("opening archive '{}'", PathWithCwd::new(&archive)))?;

    let mut exclude = args.exclude.unwrap_or_default();
    if let Some(p) = args.exclude_from {
        exclude.extend(
            read_paths(&p, args.null).with_context(|| {
                format!("reading exclude patterns from {}", PathWithCwd::new(&p))
            })?,
        );
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
        files.extend(
            read_paths(path, args.null)
                .with_context(|| format!("reading file list from {}", PathWithCwd::new(path)))?,
        );
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
            TimestampStrategy::Never,
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
        out_dir: args.out_dir,
        to_stdout: false,
        filter,
        keep_options,
        owner_options,
        same_owner: !args.no_same_owner,
        pathname_editor: PathnameEditor::new(
            args.strip_components,
            PathTransformers::new(args.substitutions, args.transforms),
            false,
        ),
        path_locks: Arc::new(PathLocks::default()),
        unlink_first: false,
    };
    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(&working_dir)
            .with_context(|| format!("changing directory to {}", PathWithCwd::new(&working_dir)))?;
    }
    apply_chroot(args.chroot)?;
    #[cfg(not(feature = "memmap"))]
    run_extract_archive_reader(
        archives
            .into_iter()
            .map(|it| io::BufReader::with_capacity(64 * 1024, it)),
        files,
        || password.as_deref(),
        output_options,
    )
    .with_context(|| format!("extracting entries from '{}'", PathWithCwd::new(&archive)))?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()
        .with_context(|| format!("memory-mapping archive '{}'", PathWithCwd::new(&archive)))?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    #[cfg(feature = "memmap")]
    run_extract_archive(archives, files, || password.as_deref(), output_options)
        .with_context(|| format!("extracting entries from '{}'", PathWithCwd::new(&archive)))?;
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
    pub(crate) out_dir: Option<PathBuf>,
    pub(crate) to_stdout: bool,
    pub(crate) filter: PathFilter<'a>,
    pub(crate) keep_options: KeepOptions,
    pub(crate) owner_options: OwnerOptions,
    pub(crate) same_owner: bool,
    pub(crate) pathname_editor: PathnameEditor,
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
    let mut globs = GlobPatterns::new(patterns.iter().map(|it| it.as_str()))
        .with_context(|| "building inclusion patterns")?;

    let mut link_entries = Vec::new();

    let (tx, rx) = std::sync::mpsc::channel();
    rayon::scope_fifo(|s| -> anyhow::Result<()> {
        run_process_archive(reader, password_provider, |entry| {
            let item = entry
                .map_err(|e| io::Error::new(e.kind(), format!("reading archive entry: {e}")))?;
            let item_path = item.name().to_string();
            if !globs.is_empty() && !globs.matches_any(&item_path) {
                log::debug!("Skip: {item_path}");
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
                tx.send(
                    extract_entry(item, password, &args)
                        .with_context(|| format!("extracting {}", item_path)),
                )
                .unwrap_or_else(|e| log::error!("{e}: {item_path}"));
            });
            Ok(())
        })
        .with_context(|| "streaming archive entries")?;
        drop(tx);
        Ok(())
    })?;
    for result in rx {
        result?;
    }
    for item in link_entries {
        let path = item.name().to_string();
        extract_entry(item, password, &args)
            .with_context(|| format!("extracting deferred link {}", path))?;
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
        let mut globs = GlobPatterns::new(files.iter().map(|it| it.as_str()))
            .with_context(|| "building inclusion patterns")?;

        let mut link_entries = Vec::<NormalEntry>::new();

        let (tx, rx) = std::sync::mpsc::channel();

        run_entries(archives, password_provider, |entry| {
            let item = entry
                .map_err(|e| io::Error::new(e.kind(), format!("reading archive entry: {e}")))?;
            let item_path = item.name().to_string();
            if !globs.is_empty() && !globs.matches_any(&item_path) {
                log::debug!("Skip: {item_path}");
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
                tx.send(
                    extract_entry(item, password, &args)
                        .with_context(|| format!("extracting {}", item_path)),
                )
                .unwrap_or_else(|e| log::error!("{e}: {item_path}"));
            });
            Ok(())
        })
        .with_context(|| "streaming archive entries")?;
        drop(tx);
        for result in rx {
            result?;
        }

        for item in link_entries {
            let path = item.name().to_string();
            extract_entry(item, password, &args)
                .with_context(|| format!("extracting deferred link {}", path))?;
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
        out_dir,
        to_stdout,
        filter,
        keep_options,
        owner_options,
        same_owner,
        pathname_editor,
        path_locks,
        unlink_first,
    }: &OutputOption<'a>,
) -> io::Result<()>
where
    T: AsRef<[u8]>,
    pna::RawChunk<T>: Chunk,
{
    if filter.excluded(item.name()) {
        return Ok(());
    }
    let item_path = item.name().as_path();
    log::debug!("Extract: {}", item_path.display());
    let Some(item_path) = pathname_editor.edit_entry_name(item_path) else {
        return Ok(());
    };

    if *to_stdout {
        return extract_entry_to_stdout(&item, password);
    }

    let path = if let Some(out_dir) = out_dir {
        Cow::from(out_dir.join(item_path))
    } else {
        Cow::from(item_path.as_path())
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
    if let Some(existing) = &metadata {
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
        utils::io::ignore_not_found(utils::fs::remove_path(&path))?;
    }

    if let Some(parent) = path.parent() {
        ensure_directory_components(parent, *unlink_first)?;
    }
    if let Some(meta) = metadata {
        if meta.is_symlink() || (meta.is_file() && entry_kind == DataKind::Directory) {
            utils::io::ignore_not_found(utils::fs::remove_path(&path))?;
        }
    }

    let remove_existing = should_overwrite_existing && !unlink_existing;

    match entry_kind {
        DataKind::File => {
            let file = utils::fs::file_create(&path, remove_existing)?;
            let mut writer = io::BufWriter::with_capacity(64 * 1024, file);
            let mut reader = item.reader(ReadOptions::with_password(password))?;
            io::copy(&mut reader, &mut writer)?;
            let mut file = writer.into_inner().map_err(|e| e.into_error())?;
            restore_timestamps(&mut file, item.metadata(), keep_options)?;
        }
        DataKind::Directory => {
            ensure_directory_components(&path, *unlink_first)?;
        }
        DataKind::SymbolicLink => {
            let reader = item.reader(ReadOptions::with_password(password))?;
            let original = io::read_to_string(reader)?;
            let original = pathname_editor.edit_symlink(original.as_ref());
            if !allow_unsafe_links && is_unsafe_link(&original) {
                log::warn!(
                    "Skipped extracting a symbolic link that contains an unsafe link. If you need to extract it, use `--allow-unsafe-links`."
                );
                return Ok(());
            }
            if remove_existing {
                utils::io::ignore_not_found(utils::fs::remove_path(&path))?;
            }
            utils::fs::symlink(original, &path)?;
        }
        DataKind::HardLink => {
            let reader = item.reader(ReadOptions::with_password(password))?;
            let original = io::read_to_string(reader)?;
            let Some(original) = pathname_editor.edit_hardlink(original.as_ref()) else {
                log::warn!(
                    "Skipped extracting a hard link that pointed at a file which was skipped.: {}",
                    original
                );
                return Ok(());
            };
            if !allow_unsafe_links && is_unsafe_link(&original) {
                log::warn!(
                    "Skipped extracting a hard link that contains an unsafe link. If you need to extract it, use `--allow-unsafe-links`."
                );
                return Ok(());
            }
            let original = if let Some(out_dir) = out_dir {
                Cow::from(out_dir.join(original))
            } else {
                Cow::from(original.as_path())
            };
            if remove_existing {
                utils::io::ignore_not_found(utils::fs::remove_path(&path))?;
            }
            fs::hard_link(original, &path)?;
        }
    }
    restore_metadata(&item, &path, keep_options, owner_options, same_owner)?;
    drop(path_guard);
    log::debug!("end: {}", path.display());
    Ok(())
}

/// Applies preserved timestamps from archive metadata to an open output file when timestamp preservation is enabled.
///
/// If the configured timestamp strategy is `Always`, sets the file's accessed and modified times (and on supported platforms, created time)
/// from the provided archive `metadata`. No changes are made when another timestamp strategy is configured.
#[inline]
fn restore_timestamps(
    file: &mut fs::File,
    metadata: &pna::Metadata,
    keep_options: &KeepOptions,
) -> io::Result<()> {
    if let TimestampStrategy::Always = keep_options.timestamp_strategy {
        let mut times = fs::FileTimes::new();
        if let Some(accessed) = metadata.accessed_time() {
            times = times.set_accessed(accessed);
        }
        if let Some(modified) = metadata.modified_time() {
            times = times.set_modified(modified);
        }
        #[cfg(any(windows, target_os = "macos"))]
        if let Some(created) = metadata.created_time() {
            times = times.set_created(created);
        }
        file.set_times(times)?;
    }
    Ok(())
}

/// Restores file metadata (permissions, extended attributes, and ACLs) for an extracted entry according to the provided keep and owner options.
///
/// Permissions are applied when `keep_options.permission_strategy` is `Always`. Extended attributes are applied on Unix when `keep_options.xattr_strategy` is `Always` (logs a warning if the filesystem or platform does not support xattrs). ACLs are restored when the `acl` feature is enabled and `keep_options.acl_strategy` requests them; if the `acl` feature is not compiled in but ACLs were requested, a warning is emitted.
fn restore_metadata<T>(
    item: &NormalEntry<T>,
    path: &Path,
    keep_options: &KeepOptions,
    owner_options: &OwnerOptions,
    same_owner: &bool,
) -> io::Result<()>
where
    T: AsRef<[u8]>,
{
    if let PermissionStrategy::Always = keep_options.permission_strategy {
        if let Some(p) = item.metadata().permission() {
            restore_permissions(*same_owner, path, p, owner_options)?;
        }
    }
    #[cfg(unix)]
    if let XattrStrategy::Always = keep_options.xattr_strategy {
        match utils::os::unix::fs::xattrs::set_xattrs(path, item.xattrs()) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::Unsupported => {
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
    restore_acls(path, item.acl()?, keep_options.acl_strategy)?;
    #[cfg(not(feature = "acl"))]
    if let AclStrategy::Always = keep_options.acl_strategy {
        log::warn!("Please enable `acl` feature and rebuild and install pna.");
    }
    Ok(())
}

/// Restore POSIX/Windows ACLs on a filesystem path when ACL preservation is enabled.
///
/// When `acl_strategy` is `AclStrategy::Always`, selects the ACL entries that match the current
/// platform if present, otherwise uses the first available platform-tagged ACL set, and applies
/// them to `path`. Empty ACL lists are ignored. On platforms without ACL support this emits a
/// warning instead of applying ACLs.
#[cfg(feature = "acl")]
fn restore_acls(path: &Path, acls: Acls, acl_strategy: AclStrategy) -> io::Result<()> {
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "macos",
        windows
    ))]
    if let AclStrategy::Always = acl_strategy {
        use crate::chunk::{AcePlatform, Acl, acl_convert_current_platform};
        use itertools::Itertools;

        let platform = AcePlatform::CURRENT;
        if let Some((platform, acl)) = acls.into_iter().find_or_first(|(p, _)| p.eq(&platform)) {
            if !acl.is_empty() {
                utils::acl::set_facl(
                    path,
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
    if let AclStrategy::Always = acl_strategy {
        log::warn!("Currently acl is not supported on this platform.");
    }
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

fn extract_entry_to_stdout<T>(item: &NormalEntry<T>, password: Option<&[u8]>) -> io::Result<()>
where
    T: AsRef<[u8]>,
    pna::RawChunk<T>: Chunk,
{
    if !matches!(item.header().data_kind(), DataKind::File) {
        return Ok(());
    }

    let mut reader = item.reader(ReadOptions::with_password(password))?;
    let mut stdout = io::stdout().lock();
    io::copy(&mut reader, &mut stdout)?;
    stdout.flush()?;
    Ok(())
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
