#[cfg(feature = "memmap")]
use crate::command::core::run_entries;
use crate::ext::*;
#[cfg(any(unix, windows))]
use crate::utils::fs::lchown;
use crate::{
    cli::{DateTime, FileArgsCompat, PasswordArgs},
    command::{
        Command, ask_password,
        core::{
            AclStrategy, FflagsStrategy, KeepOptions, MacMetadataStrategy, ModeStrategy,
            OwnerOptions, OwnerStrategy, PathFilter, PathTransformers, PathnameEditor,
            PermissionStrategyResolver, SafeWriter, TimeFilterResolver, TimeFilters,
            TimestampStrategy, TimestampStrategyResolver, Umask, XattrStrategy, apply_chroot,
            collect_split_archives,
            path_lock::PathLocks,
            re::{bsd::SubstitutionRule, gnu::TransformRule},
            read_paths, run_process_archive,
        },
    },
    utils::{
        self, BsdGlobMatcher, PathWithCwd, VCS_FILES,
        fmt::DurationDisplay,
        fs::{Group, User},
    },
};
use anyhow::Context;
use clap::{ArgGroup, Parser, ValueHint};
use pna::{DataKind, EntryName, EntryReference, NormalEntry, Permission, ReadOptions, prelude::*};
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
    group(ArgGroup::new("ctime-older-than-source").args(["older_ctime", "older_ctime_than"])),
    group(ArgGroup::new("ctime-newer-than-source").args(["newer_ctime", "newer_ctime_than"])),
    group(ArgGroup::new("mtime-older-than-source").args(["older_mtime", "older_mtime_than"])),
    group(ArgGroup::new("mtime-newer-than-source").args(["newer_mtime", "newer_mtime_than"])),
    group(
        ArgGroup::new("overwrite-flag")
            .args(["overwrite", "no_overwrite", "keep_newer_files", "keep_old_files"])
    ),
    group(ArgGroup::new("safe-writes-flag").args(["safe_writes", "no_safe_writes"])),
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
        help_heading = "Unstable Options",
        help = "Skip extracting files if a newer version already exists"
    )]
    keep_newer_files: bool,
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Skip extracting files if they already exist"
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
        value_name = "DATETIME",
        help = "Overrides the modification time"
    )]
    mtime: Option<DateTime>,
    #[arg(
        long,
        requires = "mtime",
        help = "Clamp the modification time of the entries to the specified time by --mtime"
    )]
    clamp_mtime: bool,
    #[arg(long, value_name = "DATETIME", help = "Overrides the creation time")]
    ctime: Option<DateTime>,
    #[arg(
        long,
        requires = "ctime",
        help = "Clamp the creation time of the entries to the specified time by --ctime"
    )]
    clamp_ctime: bool,
    #[arg(long, value_name = "DATETIME", help = "Overrides the access time")]
    atime: Option<DateTime>,
    #[arg(
        long,
        requires = "atime",
        help = "Clamp the access time of the entries to the specified time by --atime"
    )]
    clamp_atime: bool,
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
        help_heading = "Unstable Options",
        help = "Restore ACLs"
    )]
    keep_acl: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-acls",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Do not restore ACLs. This is the inverse option of --keep-acl"
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
        value_name = "DATETIME",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories older than the specified date. This compares ctime entries."
    )]
    older_ctime: Option<DateTime>,
    #[arg(
        long,
        value_name = "DATETIME",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories older than the specified date. This compares mtime entries."
    )]
    older_mtime: Option<DateTime>,
    #[arg(
        long,
        value_name = "DATETIME",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories newer than the specified date. This compares ctime entries."
    )]
    newer_ctime: Option<DateTime>,
    #[arg(
        long,
        value_name = "DATETIME",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories newer than the specified date. This compares mtime entries."
    )]
    newer_mtime: Option<DateTime>,
    #[arg(
        long,
        value_name = "file",
        requires = "unstable",
        visible_alias = "newer-than",
        help_heading = "Unstable Options",
        help = "Only include files and directories newer than the specified file. This compares ctime entries."
    )]
    newer_ctime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "file",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories newer than the specified file. This compares mtime entries."
    )]
    newer_mtime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "file",
        requires = "unstable",
        visible_alias = "older-than",
        help_heading = "Unstable Options",
        help = "Only include files and directories older than the specified file. This compares ctime entries."
    )]
    older_ctime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "file",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories older than the specified file. This compares mtime entries."
    )]
    older_mtime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "PATTERN",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions"
    )]
    include: Vec<String>,
    #[arg(
        long,
        value_name = "PATTERN",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Exclude path glob",
        value_hint = ValueHint::AnyPath
    )]
    exclude: Vec<String>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Read exclude files from given path",
        value_hint = ValueHint::FilePath
    )]
    exclude_from: Option<PathBuf>,
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`)"
    )]
    exclude_vcs: bool,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Read extraction patterns from given path",
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
        help_heading = "Unstable Options",
        help = "Modify file or archive member names according to pattern that like BSD tar -s option"
    )]
    substitutions: Option<Vec<SubstitutionRule>>,
    #[arg(
        long = "transform",
        visible_alias = "xform",
        value_name = "PATTERN",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Modify file or archive member names according to pattern that like GNU tar -transform option"
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
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Extract files atomically via temp file and rename"
    )]
    safe_writes: bool,
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Disable atomic extraction. This is the inverse option of --safe-writes"
    )]
    no_safe_writes: bool,
    #[command(flatten)]
    pub(crate) file: FileArgsCompat,
}

impl Command for ExtractCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        extract_archive(self)
    }
}
#[hooq::hooq(anyhow)]
fn extract_archive(args: ExtractCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password).with_context(|| "reading password")?;
    let start = Instant::now();
    let archive = args.file.archive();
    log::info!("Extract archive {}", PathWithCwd::new(&archive));

    let archives = collect_split_archives(&archive)
        .with_context(|| format!("opening archive '{}'", PathWithCwd::new(&archive)))?;

    let mut exclude = args.exclude;
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
        args.include.iter().map(|s| s.as_str()),
        exclude.iter().map(|s| s.as_str()).chain(vcs_patterns),
    );

    let mut files = args.file.files();
    if let Some(path) = &args.files_from {
        files.extend(
            read_paths(path, args.null)
                .with_context(|| format!("reading file list from {}", PathWithCwd::new(path)))?,
        );
    }

    let time_filters = TimeFilterResolver {
        newer_ctime_than: args.newer_ctime_than.as_deref(),
        older_ctime_than: args.older_ctime_than.as_deref(),
        newer_ctime: args.newer_ctime.map(|it| it.to_system_time()),
        older_ctime: args.older_ctime.map(|it| it.to_system_time()),
        newer_mtime_than: args.newer_mtime_than.as_deref(),
        older_mtime_than: args.older_mtime_than.as_deref(),
        newer_mtime: args.newer_mtime.map(|it| it.to_system_time()),
        older_mtime: args.older_mtime.map(|it| it.to_system_time()),
    }
    .resolve()?;
    let overwrite_strategy = OverwriteStrategy::from_flags(
        args.overwrite,
        args.no_overwrite,
        args.keep_newer_files,
        args.keep_old_files,
        OverwriteStrategy::Never,
    );
    let (mode_strategy, owner_strategy) = PermissionStrategyResolver {
        keep_permission: args.keep_permission,
        no_keep_permission: args.no_keep_permission,
        same_owner: !args.no_same_owner,
        uname: args.uname,
        gname: args.gname,
        uid: args.uid,
        gid: args.gid,
        numeric_owner: args.numeric_owner,
    }
    .resolve();
    let keep_options = KeepOptions {
        timestamp_strategy: TimestampStrategyResolver {
            keep_timestamp: args.keep_timestamp,
            no_keep_timestamp: args.no_keep_timestamp,
            default_preserve: false,
            mtime: args.mtime.map(|it| it.to_system_time()),
            clamp_mtime: args.clamp_mtime,
            ctime: args.ctime.map(|it| it.to_system_time()),
            clamp_ctime: args.clamp_ctime,
            atime: args.atime.map(|it| it.to_system_time()),
            clamp_atime: args.clamp_atime,
        }
        .resolve(),
        mode_strategy,
        owner_strategy,
        xattr_strategy: XattrStrategy::from_flags(args.keep_xattr, args.no_keep_xattr),
        acl_strategy: AclStrategy::from_flags(args.keep_acl, args.no_keep_acl),
        fflags_strategy: FflagsStrategy::Never,
        mac_metadata_strategy: MacMetadataStrategy::Never,
    };
    let output_options = OutputOption {
        overwrite_strategy,
        allow_unsafe_links: args.allow_unsafe_links,
        out_dir: args.out_dir,
        to_stdout: false,
        filter,
        keep_options,
        pathname_editor: PathnameEditor::new(
            args.strip_components,
            PathTransformers::new(args.substitutions, args.transforms),
            false,
        ),
        path_locks: Arc::new(PathLocks::default()),
        unlink_first: false,
        time_filters,
        safe_writes: args.safe_writes && !args.no_safe_writes,
        verbose: false,
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
        true,
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
    run_extract_archive(
        archives,
        files,
        || password.as_deref(),
        output_options,
        true,
    )
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
    pub(crate) pathname_editor: PathnameEditor,
    pub(crate) path_locks: Arc<PathLocks>,
    pub(crate) unlink_first: bool,
    pub(crate) time_filters: TimeFilters,
    pub(crate) safe_writes: bool,
    pub(crate) verbose: bool,
}

pub(crate) fn run_extract_archive_reader<'a, 'p, Provider>(
    reader: impl IntoIterator<Item = impl Read> + Send,
    files: Vec<String>,
    mut password_provider: Provider,
    args: OutputOption<'a>,
    no_recursive: bool,
) -> anyhow::Result<()>
where
    Provider: FnMut() -> Option<&'p [u8]> + Send,
{
    let password = password_provider();
    let patterns = files;
    let mut globs =
        BsdGlobMatcher::new(patterns.iter().map(|it| it.as_str())).with_no_recursive(no_recursive);

    let mut link_entries = Vec::new();

    let (tx, rx) = std::sync::mpsc::channel();
    rayon::scope_fifo(|s| -> anyhow::Result<()> {
        run_process_archive(reader, password_provider, |entry| {
            let item = entry
                .map_err(|e| io::Error::new(e.kind(), format!("reading archive entry: {e}")))?;
            let Some(name) = filter_entry(&item, &mut globs, &args) else {
                return Ok(());
            };
            if args.verbose {
                eprintln!("x {}", name);
            }
            if args.to_stdout {
                return extract_entry_to_stdout(&item, password);
            }
            if matches!(
                item.header().data_kind(),
                DataKind::SymbolicLink | DataKind::HardLink
            ) {
                link_entries.push((name, item));
                return Ok(());
            }
            let item_path = item.name().to_string();
            let tx = tx.clone();
            let args = args.clone();
            s.spawn_fifo(move |_| {
                tx.send(
                    extract_entry(item, &name, password, &args)
                        .with_context(|| format!("extracting {}", item_path)),
                )
                .unwrap_or_else(|_| unreachable!("receiver is held by scope owner"));
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
    for (name, item) in link_entries {
        extract_entry(item, &name, password, &args)
            .with_context(|| format!("extracting deferred link {name}"))?;
    }

    globs.ensure_all_matched()?;
    Ok(())
}

#[cfg(feature = "memmap")]
#[hooq::hooq(anyhow)]
pub(crate) fn run_extract_archive<'a, 'd, 'p, Provider>(
    archives: impl IntoIterator<Item = &'d [u8]> + Send,
    files: Vec<String>,
    mut password_provider: Provider,
    args: OutputOption<'a>,
    no_recursive: bool,
) -> anyhow::Result<()>
where
    Provider: FnMut() -> Option<&'p [u8]> + Send,
{
    rayon::scope_fifo(|s| -> anyhow::Result<()> {
        let password = password_provider();
        let mut globs =
            BsdGlobMatcher::new(files.iter().map(|it| it.as_str())).with_no_recursive(no_recursive);

        let mut link_entries = Vec::<(_, NormalEntry)>::new();

        let (tx, rx) = std::sync::mpsc::channel();

        #[hooq::skip_all]
        run_entries(archives, password_provider, |entry| {
            let item = entry
                .map_err(|e| io::Error::new(e.kind(), format!("reading archive entry: {e}")))?;
            let Some(name) = filter_entry(&item, &mut globs, &args) else {
                return Ok(());
            };
            if args.verbose {
                eprintln!("x {}", name);
            }
            if args.to_stdout {
                return extract_entry_to_stdout(&item, password);
            }
            if matches!(
                item.header().data_kind(),
                DataKind::SymbolicLink | DataKind::HardLink
            ) {
                link_entries.push((name, item.into()));
                return Ok(());
            }
            let item_path = item.name().to_string();
            let tx = tx.clone();
            let args = args.clone();
            s.spawn_fifo(move |_| {
                tx.send(
                    extract_entry(item, &name, password, &args)
                        .with_context(|| format!("extracting {}", item_path)),
                )
                .unwrap_or_else(|_| unreachable!("receiver is held by scope owner"));
            });
            Ok(())
        })
        .with_context(|| "streaming archive entries")?;
        drop(tx);
        for result in rx {
            result?;
        }

        for (name, item) in link_entries {
            extract_entry(item, &name, password, &args)
                .with_context(|| format!("extracting deferred link {name}"))?;
        }
        globs.ensure_all_matched()?;
        Ok(())
    })
}

#[inline]
fn entry_matches_time_filters<T>(item: &NormalEntry<T>, filters: &TimeFilters) -> bool
where
    T: AsRef<[u8]>,
    pna::RawChunk<T>: Chunk,
{
    let metadata = item.metadata();
    filters.matches_or_inactive(metadata.created_time(), metadata.modified_time())
}

fn filter_entry<T: AsRef<[u8]>>(
    item: &NormalEntry<T>,
    globs: &mut BsdGlobMatcher<'_>,
    args: &OutputOption<'_>,
) -> Option<EntryName>
where
    pna::RawChunk<T>: Chunk,
{
    let item_name = item.name();
    if !globs.is_empty() && !globs.matches(item_name) {
        log::debug!("Skip: {item_name}");
        return None;
    }
    if args.filter.excluded(item_name) {
        log::debug!("Skip: {item_name}");
        return None;
    }
    if !entry_matches_time_filters(item, &args.time_filters) {
        log::debug!("Skip: {item_name}");
        return None;
    }
    let name = args.pathname_editor.edit_entry_name(item_name.as_path());
    if name.is_none() {
        log::debug!("Skip: {item_name}");
    }
    name
}

/// Result of checking whether extraction should proceed for a given path.
#[derive(Debug, Clone, Copy)]
enum ExtractionDecision {
    /// Proceed with extraction; `remove_existing` indicates if existing file should be removed first
    Proceed { remove_existing: bool },
    /// Skip extraction (e.g., keep-newer/keep-older strategies)
    Skip,
}

/// Checks overwrite strategy and prepares the target path for extraction.
///
/// This function:
/// 1. Checks if a file/directory already exists at the target path
/// 2. Applies the overwrite strategy to decide whether to proceed
/// 3. Prepares parent directories
/// 4. Handles conflicts between entry types (e.g., file vs directory)
///
/// Returns `ExtractionDecision::Skip` if extraction should be skipped,
/// or `ExtractionDecision::Proceed` with information about whether to remove existing files.
fn check_and_prepare_target<T>(
    path: &Path,
    entry_kind: DataKind,
    item: &NormalEntry<T>,
    overwrite_strategy: OverwriteStrategy,
    unlink_first: bool,
) -> io::Result<ExtractionDecision>
where
    T: AsRef<[u8]>,
{
    let metadata = match fs::symlink_metadata(path) {
        Ok(meta) => Some(meta),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };

    // Check overwrite strategy
    if let Some(existing) = &metadata {
        match overwrite_strategy {
            OverwriteStrategy::Never if !unlink_first => {
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
                return Ok(ExtractionDecision::Skip);
            }
            OverwriteStrategy::KeepNewer => {
                if is_existing_newer(existing, item) {
                    log::debug!(
                        "Skipped extracting {}: newer one already exists (--keep-newer)",
                        path.display()
                    );
                    return Ok(ExtractionDecision::Skip);
                }
            }
            OverwriteStrategy::Always | OverwriteStrategy::Never => (),
        }
    }

    // Determine what cleanup is needed
    let (had_existing, existing_is_dir) = metadata
        .as_ref()
        .map(|meta| (true, meta.is_dir()))
        .unwrap_or((false, false));
    let unlink_existing =
        unlink_first && had_existing && (entry_kind != DataKind::Directory || !existing_is_dir);
    let should_overwrite_existing = matches!(
        overwrite_strategy,
        OverwriteStrategy::Always | OverwriteStrategy::KeepNewer
    ) && had_existing;

    // Remove existing if unlink_first mode
    if unlink_existing {
        utils::io::ignore_not_found(utils::fs::remove_path(path))?;
    }

    // Create parent directories
    if let Some(parent) = path.parent() {
        ensure_directory_components(parent, unlink_first)?;
    }

    // Handle type conflicts (symlink blocking file, file blocking directory)
    if let Some(meta) = metadata
        && (meta.is_symlink() || (meta.is_file() && entry_kind == DataKind::Directory))
    {
        utils::io::ignore_not_found(utils::fs::remove_path(path))?;
    }

    let remove_existing = should_overwrite_existing && !unlink_existing;
    Ok(ExtractionDecision::Proceed { remove_existing })
}

pub(crate) fn extract_entry<'a, T>(
    item: NormalEntry<T>,
    item_path: &EntryName,
    password: Option<&'a [u8]>,
    OutputOption {
        overwrite_strategy,
        allow_unsafe_links,
        out_dir,
        to_stdout: _,
        filter: _,
        keep_options,
        pathname_editor,
        path_locks,
        unlink_first,
        time_filters: _,
        safe_writes,
        verbose: _,
    }: &OutputOption<'a>,
) -> io::Result<()>
where
    T: AsRef<[u8]>,
    pna::RawChunk<T>: Chunk,
{
    log::debug!("Extract: {}", item.name());
    let path = build_output_path(out_dir.as_deref(), item_path.as_path());

    let entry_kind = item.header().data_kind();

    let path_lock = path_locks.get(path.as_ref());
    let path_guard = path_lock.lock().expect("path lock mutex poisoned");

    log::debug!("start: {}", path.display());

    // Check overwrite strategy and prepare target
    let ExtractionDecision::Proceed { remove_existing } =
        check_and_prepare_target(&path, entry_kind, &item, *overwrite_strategy, *unlink_first)?
    else {
        return Ok(());
    };

    match entry_kind {
        DataKind::File => {
            if *safe_writes {
                let mut safe_writer = SafeWriter::new(&path)?;
                {
                    let mut writer =
                        io::BufWriter::with_capacity(64 * 1024, safe_writer.as_file_mut());
                    let mut reader = item.reader(ReadOptions::with_password(password))?;
                    io::copy(&mut reader, &mut writer)?;
                    writer.flush()?;
                }
                // Set timestamps before persist; after rename we lose the file handle
                restore_timestamps(safe_writer.as_file_mut(), item.metadata(), keep_options)?;
                safe_writer.persist()?;
            } else {
                if remove_existing {
                    utils::io::ignore_not_found(utils::fs::remove_path(&path))?;
                }
                let file = utils::fs::file_create(&path, remove_existing)?;
                let mut writer = io::BufWriter::with_capacity(64 * 1024, file);
                let mut reader = item.reader(ReadOptions::with_password(password))?;
                io::copy(&mut reader, &mut writer)?;
                let mut file = writer.into_inner().map_err(|e| e.into_error())?;
                restore_timestamps(&mut file, item.metadata(), keep_options)?;
            }
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
            // Symlinks/hardlinks cannot be atomically replaced; remove existing path first
            if *safe_writes || remove_existing {
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
            // Symlinks/hardlinks cannot be atomically replaced; remove existing path first
            if *safe_writes || remove_existing {
                utils::io::ignore_not_found(utils::fs::remove_path(&path))?;
            }
            fs::hard_link(original, &path)?;
        }
    }
    restore_metadata(&item, &path, keep_options)?;
    drop(path_guard);
    log::debug!("end: {}", path.display());
    Ok(())
}

#[inline]
fn build_output_path<'a>(out_dir: Option<&'a Path>, item_path: &'a Path) -> Cow<'a, Path> {
    let path = if let Some(out_dir) = out_dir {
        Cow::from(out_dir.join(item_path))
    } else {
        Cow::Borrowed(item_path)
    };
    if path.as_os_str().is_empty() {
        Cow::Borrowed(".".as_ref())
    } else {
        path
    }
}

/// Applies preserved timestamps from archive metadata to an open output file when timestamp preservation is enabled.
///
/// When the configured timestamp strategy is enabled, sets the file's accessed and modified times (and on supported platforms, created time)
/// from the provided archive `metadata`. Timestamps may be overridden or clamped according to the strategy's configuration.
/// No changes are made when timestamp strategy is disabled.
#[inline]
fn restore_timestamps(
    file: &mut fs::File,
    metadata: &pna::Metadata,
    keep_options: &KeepOptions,
) -> io::Result<()> {
    if let TimestampStrategy::Preserve {
        mtime,
        ctime: _ctime,
        atime,
    } = keep_options.timestamp_strategy
    {
        let mut times = fs::FileTimes::new();
        if let Some(accessed) = atime.resolve(metadata.accessed_time()) {
            times = times.set_accessed(accessed);
        }
        if let Some(modified) = mtime.resolve(metadata.modified_time()) {
            times = times.set_modified(modified);
        }
        #[cfg(any(windows, target_os = "macos"))]
        if let Some(created) = _ctime.resolve(metadata.created_time()) {
            times = times.set_created(created);
        }
        file.set_times(times)?;
    }
    Ok(())
}

/// Restores file metadata (permissions, extended attributes, ACLs, and macOS metadata) for an extracted entry according to the provided keep options.
///
/// - Ownership is restored when `owner_strategy` is `Preserve`
/// - Mode bits are restored when `mode_strategy` is `Preserve`
/// - These are independent: `--keep-permission --no-same-owner` restores mode but not ownership
fn restore_metadata<T>(
    item: &NormalEntry<T>,
    path: &Path,
    keep_options: &KeepOptions,
) -> io::Result<()>
where
    T: AsRef<[u8]>,
{
    if let Some(p) = item.metadata().permission() {
        // Restore ownership when owner_strategy is Preserve (independent of mode)
        if let OwnerStrategy::Preserve { options } = &keep_options.owner_strategy {
            restore_owner(path, p, options)?;
        }
        // Restore mode bits when configured.
        match keep_options.mode_strategy {
            ModeStrategy::Preserve => restore_mode(path, p)?,
            ModeStrategy::Masked(mask) => restore_mode_masked(path, p, mask)?,
            ModeStrategy::Never => {}
        }
    }
    // On macOS, when mac_metadata_strategy is Always and the entry has mac_metadata,
    // AppleDouble restoration via copyfile() will include xattrs and ACLs.
    // Skip separate handling to avoid duplication.
    #[cfg(target_os = "macos")]
    let skip_xattr_acl = matches!(
        keep_options.mac_metadata_strategy,
        MacMetadataStrategy::Always
    ) && item.mac_metadata().is_some();
    #[cfg(not(target_os = "macos"))]
    let skip_xattr_acl = false;

    #[cfg(unix)]
    if !skip_xattr_acl {
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
    }
    #[cfg(not(unix))]
    if let XattrStrategy::Always = keep_options.xattr_strategy {
        log::warn!("Currently extended attribute is not supported on this platform.");
    }
    #[cfg(feature = "acl")]
    if !skip_xattr_acl {
        restore_acls(path, item.acl()?, keep_options.acl_strategy)?;
    }
    #[cfg(not(feature = "acl"))]
    if let AclStrategy::Always = keep_options.acl_strategy {
        log::warn!("Please enable `acl` feature and rebuild and install pna.");
    }
    if let FflagsStrategy::Always = keep_options.fflags_strategy {
        let flags = item.fflags();
        if !flags.is_empty() {
            match utils::fs::set_flags(path, &flags) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::Unsupported => {
                    log::warn!(
                        "File flags are not supported on filesystem for '{}': {}",
                        path.display(),
                        e
                    );
                }
                Err(e) => return Err(e),
            }
        }
    }
    // macOS metadata (AppleDouble) - restores xattrs, ACLs, resource forks via copyfile()
    #[cfg(target_os = "macos")]
    if let MacMetadataStrategy::Always = keep_options.mac_metadata_strategy {
        if let Some(apple_double_data) = item.mac_metadata() {
            match utils::os::unix::fs::copyfile::unpack_apple_double(apple_double_data, path) {
                Ok(()) => {
                    log::debug!("Unpacked macOS metadata for '{}'", path.display());
                }
                Err(e) => {
                    log::warn!(
                        "Failed to restore macOS metadata for '{}': {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    if let MacMetadataStrategy::Always = keep_options.mac_metadata_strategy {
        if item.mac_metadata().is_some() {
            log::warn!(
                "macOS metadata present but cannot be restored on this platform: '{}'",
                path.display()
            );
        }
    }
    Ok(())
}

/// Restore POSIX/Windows ACLs on a filesystem path when ACL preservation is enabled.
///
/// When `acl_strategy` is `AclStrategy::Always`, selects the ACL entries that match the current
/// platform if present, otherwise uses the first available platform-tagged ACL set, and applies
/// them to `path`. Empty ACL lists are ignored.
///
/// On platforms without ACL support, this emits a warning and returns successfully.
/// On supported platforms, if the target filesystem does not support ACLs (e.g., FAT32),
/// a warning is logged for that path and the operation continues.
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
                match utils::acl::set_facl(
                    path,
                    acl_convert_current_platform(Acl {
                        platform,
                        entries: acl,
                    }),
                ) {
                    Ok(()) => {}
                    Err(e) if e.kind() == io::ErrorKind::Unsupported => {
                        log::warn!(
                            "ACL not supported on this filesystem, skipping '{}': {}",
                            path.display(),
                            e
                        );
                    }
                    Err(e) => return Err(e),
                }
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

/// Resolves the user and group to use for ownership restoration.
///
/// Priority:
/// 1. Override uid/gid if specified
/// 2. Override uname/gname if specified, searching by name
/// 3. Archive's uname/gname with fallback to archive's uid/gid
fn resolve_owner(
    permission: &Permission,
    uname_override: Option<&str>,
    gname_override: Option<&str>,
    uid_override: Option<u32>,
    gid_override: Option<u32>,
) -> (Option<User>, Option<Group>) {
    let user = if let Some(uid) = uid_override {
        User::from_uid(uid.into()).ok()
    } else {
        let name = uname_override.unwrap_or(permission.uname());
        search_owner(name, permission.uid()).ok()
    };
    let group = if let Some(gid) = gid_override {
        Group::from_gid(gid.into()).ok()
    } else {
        let name = gname_override.unwrap_or(permission.gname());
        search_group(name, permission.gid()).ok()
    };
    (user, group)
}

/// Restores file ownership (uid/gid) for an extracted entry.
/// Called when `OwnerStrategy::Preserve` is set.
#[inline]
fn restore_owner(path: &Path, p: &Permission, options: &OwnerOptions) -> io::Result<()> {
    #[cfg(any(unix, windows))]
    {
        let (user, group) = resolve_owner(
            p,
            options.uname.as_deref(),
            options.gname.as_deref(),
            options.uid,
            options.gid,
        );
        match lchown(path, user, group) {
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                log::warn!("failed to restore owner of {}: {}", path.display(), e)
            }
            r => r?,
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, p, options);
        log::warn!("Currently ownership restoration is not supported on this platform.");
    }
    Ok(())
}

/// Restores file mode bits (permissions like 0755) for an extracted entry.
/// Called when `ModeStrategy::Preserve` is set.
#[inline]
fn restore_mode(path: &Path, p: &Permission) -> io::Result<()> {
    #[cfg(any(unix, windows))]
    {
        utils::fs::chmod(path, p.permissions())?;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, p);
        log::warn!(
            "Skipping mode restoration for '{}': not supported on this platform.",
            path.display()
        );
    }
    Ok(())
}

/// Restores file mode bits with umask applied and suid/sgid/sticky cleared.
#[inline]
fn restore_mode_masked(path: &Path, p: &Permission, umask: Umask) -> io::Result<()> {
    #[cfg(any(unix, windows))]
    {
        utils::fs::chmod(path, umask.apply(p.permissions()))?;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, p, umask);
        log::warn!(
            "Skipping mode restoration for '{}': not supported on this platform.",
            path.display()
        );
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
