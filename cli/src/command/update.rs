use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, DateTime, FileArgs, HashAlgorithmArgs,
        MissingTimePolicy, PasswordArgs, SolidEntriesTransformStrategy,
        SolidEntriesTransformStrategyArgs,
    },
    command::{
        Command, ask_password, check_password,
        core::{
            AclStrategy, CollectOptions, CollectedEntry, CreateOptions, FflagsStrategy,
            KeepOptions, MacMetadataStrategy, PathFilter, PathTransformers, PathnameEditor,
            PermissionStrategyResolver, SplitArchiveReader, TimeFilterResolver,
            TimestampStrategyResolver, TransformStrategy, TransformStrategyKeepSolid,
            TransformStrategyUnSolid, XattrStrategy, collect_items_from_paths,
            collect_split_archives, create_entry, entry_option,
            iter::ReorderByIndex,
            re::{bsd::SubstitutionRule, gnu::TransformRule},
            read_paths, read_paths_stdin,
        },
    },
    utils::{PathPartExt, VCS_FILES, env::NamedTempFile, fs::HardlinkResolver},
};
use clap::{ArgGroup, Parser, ValueHint};
use indexmap::IndexMap;
use pna::{Archive, EntryName, Metadata, prelude::*};
use std::{env, fs, io, path::PathBuf};

#[derive(Parser, Clone, Debug)]
#[command(
    group(ArgGroup::new("keep-acl-flag").args(["keep_acl", "no_keep_acl"])),
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
    group(ArgGroup::new("keep-xattr-flag").args(["keep_xattr", "no_keep_xattr"])),
    group(ArgGroup::new("keep-timestamp-flag").args(["keep_timestamp", "no_keep_timestamp"])),
    group(ArgGroup::new("keep-permission-flag").args(["keep_permission", "no_keep_permission"])),
    group(ArgGroup::new("ctime-older-than-source").args(["older_ctime", "older_ctime_than"])),
    group(ArgGroup::new("ctime-newer-than-source").args(["newer_ctime", "newer_ctime_than"])),
    group(ArgGroup::new("mtime-older-than-source").args(["older_mtime", "older_mtime_than"])),
    group(ArgGroup::new("mtime-newer-than-source").args(["newer_mtime", "newer_mtime_than"])),
)]
pub(crate) struct UpdateCommand {
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Stay in the same file system when collecting files"
    )]
    one_file_system: bool,
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Exclude files with the nodump flag"
    )]
    nodump: bool,
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
    #[arg(long, help = "Include directories in archive")]
    keep_dir: bool,
    #[arg(
        long,
        help = "Do not archive directories. This is the inverse option of --keep-dir"
    )]
    no_keep_dir: bool,
    #[arg(
        long,
        visible_alias = "preserve-timestamps",
        help = "Preserve file timestamps"
    )]
    keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-timestamps",
        help = "Do not archive timestamp of files. This is the inverse option of --preserve-timestamps"
    )]
    pub(crate) no_keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Preserve file permissions"
    )]
    #[cfg_attr(windows, arg(requires = "unstable", help_heading = "Unstable Options"))]
    keep_permission: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-permissions",
        help = "Do not archive permissions of files. This is the inverse option of --preserve-permissions"
    )]
    #[cfg_attr(windows, arg(requires = "unstable", help_heading = "Unstable Options"))]
    no_keep_permission: bool,
    #[arg(
        long,
        visible_alias = "preserve-xattrs",
        help = "Preserve extended attributes"
    )]
    keep_xattr: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-xattrs",
        help = "Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs"
    )]
    pub(crate) no_keep_xattr: bool,
    #[arg(
        long,
        visible_alias = "preserve-acls",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Preserve ACLs"
    )]
    keep_acl: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-acls",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Do not archive ACLs. This is the inverse option of --keep-acl"
    )]
    no_keep_acl: bool,
    #[arg(long, value_name = "NAME", help = "Set user name for archive entries")]
    uname: Option<String>,
    #[arg(long, value_name = "NAME", help = "Set group name for archive entries")]
    gname: Option<String>,
    #[arg(
        long,
        value_name = "ID",
        help = "Overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id"
    )]
    uid: Option<u32>,
    #[arg(
        long,
        value_name = "ID",
        help = "Overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id"
    )]
    gid: Option<u32>,
    #[arg(
        long,
        value_name = "N",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Remove the specified number of leading path elements when storing paths"
    )]
    strip_components: Option<usize>,
    #[arg(
        long,
        help = "This is equivalent to --uname \"\" --gname \"\". It causes user and group names to not be stored in the archive"
    )]
    numeric_owner: bool,
    #[arg(
        long,
        value_name = "DATETIME",
        help = "Overrides the creation time read from disk"
    )]
    ctime: Option<DateTime>,
    #[arg(
        long,
        requires = "ctime",
        help = "Clamp the creation time of the entries to the specified time by --ctime"
    )]
    clamp_ctime: bool,
    #[arg(
        long,
        value_name = "DATETIME",
        help = "Overrides the access time read from disk"
    )]
    atime: Option<DateTime>,
    #[arg(
        long,
        requires = "atime",
        help = "Clamp the access time of the entries to the specified time by --atime"
    )]
    clamp_atime: bool,
    #[arg(
        long,
        value_name = "DATETIME",
        help = "Overrides the modification time read from disk"
    )]
    mtime: Option<DateTime>,
    #[arg(
        long,
        requires = "mtime",
        help = "Clamp the modification time of the entries to the specified time by --mtime"
    )]
    clamp_mtime: bool,
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
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories newer than the specified file. This compares ctime entries."
    )]
    newer_ctime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories newer than the specified file. This compares mtime entries."
    )]
    newer_mtime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories older than the specified file. This compares ctime entries."
    )]
    older_ctime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Only include files and directories older than the specified file. This compares mtime entries."
    )]
    older_mtime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Read archiving files from given path",
        value_hint = ValueHint::FilePath
    )]
    files_from: Option<PathBuf>,
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Read archiving files from stdin"
    )]
    files_from_stdin: bool,
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
        short = 'C',
        long = "cd",
        visible_aliases = ["directory"],
        value_name = "DIRECTORY",
        help = "Change directory before adding the following files",
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
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Ignore files from .gitignore"
    )]
    gitignore: bool,
    #[arg(long, visible_aliases = ["dereference"], help = "Follow symbolic links")]
    follow_links: bool,
    #[arg(
        short = 'H',
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Follow symbolic links named on the command line"
    )]
    follow_command_links: bool,
    #[arg(
        long,
        help = "Synchronize archive with source: remove entries for files that no longer exist in the source"
    )]
    sync: bool,
}

impl Command for UpdateCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        update_archive(self)
    }
}

#[hooq::hooq(anyhow)]
fn update_archive(args: UpdateCommand) -> anyhow::Result<()> {
    let transform_strategy = args.transform_strategy.strategy();
    let sync = args.sync;
    let current_dir = env::current_dir()?;
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let archive_path = &args.file.archive;
    if !archive_path.exists() {
        anyhow::bail!("{} is not exists", archive_path.display());
    }
    let password = password.as_deref();
    let option = entry_option(args.compression, args.cipher, args.hash, password);
    let (mode_strategy, owner_strategy) = PermissionStrategyResolver {
        keep_permission: args.keep_permission,
        no_keep_permission: args.no_keep_permission,
        same_owner: true, // Must be `true` for creation
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
    let time_filters = TimeFilterResolver {
        newer_ctime_than: args.newer_ctime_than.as_deref(),
        older_ctime_than: args.older_ctime_than.as_deref(),
        newer_ctime: args.newer_ctime.map(|it| it.to_system_time()),
        older_ctime: args.older_ctime.map(|it| it.to_system_time()),
        newer_mtime_than: args.newer_mtime_than.as_deref(),
        older_mtime_than: args.older_mtime_than.as_deref(),
        newer_mtime: args.newer_mtime.map(|it| it.to_system_time()),
        older_mtime: args.older_mtime.map(|it| it.to_system_time()),
        missing_ctime: MissingTimePolicy::Include,
        missing_mtime: MissingTimePolicy::Include,
    }
    .resolve()?;
    let create_options = CreateOptions {
        option,
        keep_options,
        pathname_editor: PathnameEditor::new(
            args.strip_components,
            PathTransformers::new(args.substitutions, args.transforms),
            false,
        ),
    };

    let archives = collect_split_archives(&args.file.archive)?;

    let mut files = args.file.files;
    if args.files_from_stdin {
        files.extend(read_paths_stdin(args.null)?);
    } else if let Some(path) = args.files_from {
        files.extend(read_paths(path, args.null)?);
    }

    let mut exclude = args.exclude;
    if let Some(p) = args.exclude_from {
        exclude.extend(read_paths(p, args.null)?);
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

    let archive_path = current_dir.join(args.file.archive);
    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
    }

    let collect_options = CollectOptions {
        recursive: !args.no_recursive,
        keep_dir: args.keep_dir,
        gitignore: args.gitignore,
        nodump: args.nodump,
        follow_links: args.follow_links,
        follow_command_links: args.follow_command_links,
        one_file_system: args.one_file_system,
        filter: &filter,
        time_filters: &time_filters,
    };
    let mut resolver = HardlinkResolver::new(collect_options.follow_links);
    let target_items = collect_items_from_paths(&files, &collect_options, &mut resolver)?;

    let mut temp_file =
        NamedTempFile::new(|| archive_path.parent().unwrap_or_else(|| ".".as_ref()))?;
    let mut out_archive = Archive::write_header(temp_file.as_file_mut())?;

    let mut source = SplitArchiveReader::new(archives)?;
    match transform_strategy {
        SolidEntriesTransformStrategy::UnSolid => run_update_archive(
            &mut source,
            password,
            &create_options,
            target_items,
            sync,
            &mut out_archive,
            TransformStrategyUnSolid,
            false,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_update_archive(
            &mut source,
            password,
            &create_options,
            target_items,
            sync,
            &mut out_archive,
            TransformStrategyKeepSolid,
            false,
        ),
    }?;
    out_archive.finalize()?;
    drop(source);

    temp_file.persist(archive_path.remove_part())?;

    Ok(())
}

pub(crate) fn run_update_archive<Strategy, W>(
    source: &mut SplitArchiveReader,
    password: Option<&[u8]>,
    create_options: &CreateOptions,
    target_items: Vec<CollectedEntry>,
    sync: bool,
    out_archive: &mut Archive<W>,
    _strategy: Strategy,
    verbose: bool,
) -> anyhow::Result<()>
where
    Strategy: TransformStrategy,
    W: io::Write + Send,
{
    let (tx, rx) = std::sync::mpsc::channel();

    let mut target_files_mapping = target_items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| (EntryName::from_lossy(&item.path), (idx, item)))
        .collect::<IndexMap<_, _>>();

    rayon::scope_fifo(|s| -> anyhow::Result<()> {
        source.for_each_read_entry(|entry| {
            Strategy::transform(out_archive, password, entry, |entry| {
                let entry = entry?;
                if let Some((idx, item)) = target_files_mapping.shift_remove(entry.header().path())
                {
                    let need_update =
                        is_newer_than_archive(&item.metadata, entry.metadata()).unwrap_or(true);
                    if need_update {
                        let tx = tx.clone();
                        let create_options = create_options.clone();
                        s.spawn_fifo(move |_| {
                            log::debug!("Updating: {}", item.path.display());
                            tx.send((idx, create_entry(&item, &create_options)))
                                .unwrap_or_else(|_| {
                                    unreachable!("receiver is held by scope owner")
                                });
                        });
                        Ok(None)
                    } else {
                        Ok(Some(entry))
                    }
                } else if sync {
                    log::debug!("Removing (sync): {}", entry.header().path());
                    Ok(None)
                } else {
                    Ok(Some(entry))
                }
            })
        })?;

        // NOTE: Add new entries
        for (_, (idx, item)) in target_files_mapping {
            let tx = tx.clone();
            let create_options = create_options.clone();
            s.spawn_fifo(move |_| {
                log::debug!("Adding: {}", item.path.display());
                tx.send((idx, create_entry(&item, &create_options)))
                    .unwrap_or_else(|_| unreachable!("receiver is held by scope owner"));
            });
        }
        drop(tx);
        Ok(())
    })?;

    for entry in ReorderByIndex::new(rx.into_iter()) {
        if let Some(entry) = entry? {
            if verbose {
                eprintln!("a {}", entry.name());
            }
            out_archive.add_entry(entry)?;
        }
    }

    Ok(())
}

#[inline]
fn is_newer_than_archive(fs_meta: &fs::Metadata, metadata: &Metadata) -> Option<bool> {
    let fs_mtime = fs_meta.modified().ok()?;
    let archive_mtime = metadata.modified_time()?;
    Some(archive_mtime < fs_mtime)
}
