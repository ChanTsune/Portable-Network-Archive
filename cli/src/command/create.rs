use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, DateTime, FileArgsCompat, HashAlgorithmArgs,
        MissingTimePolicy, PasswordArgs,
    },
    command::{
        Command, ask_password, check_password,
        core::{
            AclStrategy, CollectOptions, CollectedItem, CreateOptions, EntryResult, FflagsStrategy,
            KeepOptions, MIN_SPLIT_PART_BYTES, MacMetadataStrategy, PathFilter, PathTransformers,
            PathnameEditor, PermissionStrategyResolver, TimeFilterResolver, TimeFilters,
            TimestampStrategyResolver, XattrStrategy, collect_items_from_paths,
            drain_entry_results, entry_option,
            iter::ReorderByIndex,
            re::{bsd::SubstitutionRule, gnu::TransformRule},
            read_paths, read_paths_stdin, spawn_entry_results, write_split_archive,
        },
    },
    utils::{self, VCS_FILES, fmt::DurationDisplay, fs::HardlinkResolver},
};
use anyhow::ensure;
use bytesize::ByteSize;
use clap::{ArgGroup, Parser, ValueHint};
use pna::{Archive, SolidEntryBuilder, WriteOptions};
use std::{
    env, fs,
    io::{self, prelude::*},
    path::{Path, PathBuf},
    time::Instant,
};

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
    group(ArgGroup::new("ctime-older-than-source").args(["older_ctime", "older_ctime_than"])),
    group(ArgGroup::new("ctime-newer-than-source").args(["newer_ctime", "newer_ctime_than"])),
    group(ArgGroup::new("mtime-older-than-source").args(["older_mtime", "older_mtime_than"])),
    group(ArgGroup::new("mtime-newer-than-source").args(["newer_mtime", "newer_mtime_than"])),
    group(ArgGroup::new("overwrite-flag").args(["overwrite", "no_overwrite"])),
    group(ArgGroup::new("keep-permission-flag").args(["keep_permission", "no_keep_permission"])),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission", "no_keep_permission"]).requires("unstable")),
))]
pub(crate) struct CreateCommand {
    #[arg(
        long,
        requires = "unstable",
        help = "Stay in the same file system when collecting files (unstable)"
    )]
    one_file_system: bool,
    #[arg(
        long,
        requires = "unstable",
        help = "Exclude files with the nodump flag (unstable)"
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
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[arg(
        long,
        help = "Do not overwrite files. This is the inverse option of --overwrite"
    )]
    no_overwrite: bool,
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
        help = "Preserve file permissions (unstable on Windows)"
    )]
    keep_permission: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-permissions",
        help = "Do not archive permissions of files. This is the inverse option of --preserve-permissions"
    )]
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
        help = "Preserve ACLs (unstable)"
    )]
    keep_acl: bool,
    #[arg(
        long,
        visible_alias = "no-preserve-acls",
        requires = "unstable",
        help = "Do not archive ACLs. This is the inverse option of --keep-acl (unstable)"
    )]
    no_keep_acl: bool,
    #[arg(
        long,
        value_name = "size",
        help = "Splits archive by given size in bytes (minimum 64B)"
    )]
    pub(crate) split: Option<Option<ByteSize>>,
    #[arg(
        long,
        help = "Compress multiple files together for better compression ratio"
    )]
    solid: bool,
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
        help = "Remove the specified number of leading path elements when storing paths (unstable)"
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
        help = "Only include files and directories older than the specified date (unstable). This compares ctime entries."
    )]
    older_ctime: Option<DateTime>,
    #[arg(
        long,
        value_name = "DATETIME",
        requires = "unstable",
        help = "Only include files and directories older than the specified date (unstable). This compares mtime entries."
    )]
    older_mtime: Option<DateTime>,
    #[arg(
        long,
        value_name = "DATETIME",
        requires = "unstable",
        help = "Only include files and directories newer than the specified date (unstable). This compares ctime entries."
    )]
    newer_ctime: Option<DateTime>,
    #[arg(
        long,
        value_name = "DATETIME",
        requires = "unstable",
        help = "Only include files and directories newer than the specified date (unstable). This compares mtime entries."
    )]
    newer_mtime: Option<DateTime>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help = "Only include files and directories newer than the specified file (unstable). This compares ctime entries."
    )]
    newer_ctime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help = "Only include files and directories newer than the specified file (unstable). This compares mtime entries."
    )]
    newer_mtime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help = "Only include files and directories older than the specified file (unstable). This compares ctime entries."
    )]
    older_ctime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help = "Only include files and directories older than the specified file (unstable). This compares mtime entries."
    )]
    older_mtime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help = "Read archiving files from given path (unstable)",
        value_hint = ValueHint::FilePath
    )]
    files_from: Option<PathBuf>,
    #[arg(
        long,
        requires = "unstable",
        help = "Read archiving files from stdin (unstable)"
    )]
    files_from_stdin: bool,
    #[arg(
        long,
        value_name = "PATTERN",
        requires = "unstable",
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)"
    )]
    include: Vec<String>,
    #[arg(
        long,
        value_name = "PATTERN",
        requires = "unstable",
        help = "Exclude path glob (unstable)",
        value_hint = ValueHint::AnyPath
    )]
    exclude: Vec<String>,
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
        requires = "unstable",
        help = "Ignore files from .gitignore (unstable)"
    )]
    gitignore: bool,
    #[arg(long, visible_aliases = ["dereference"], help = "Follow symbolic links")]
    follow_links: bool,
    #[arg(
        short = 'H',
        long,
        requires = "unstable",
        help = "Follow symbolic links named on the command line (unstable)"
    )]
    follow_command_links: bool,
    #[arg(
        long,
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
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
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) hash: HashAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgsCompat,
}

impl Command for CreateCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        create_archive(self)
    }
}

#[hooq::hooq(anyhow)]
fn create_archive(args: CreateCommand) -> anyhow::Result<()> {
    let current_dir = env::current_dir()?;
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let start = Instant::now();
    let archive = &args.file.archive();
    let max_file_size = args
        .split
        .map(|opt| {
            usize::try_from(opt.unwrap_or(ByteSize::gb(1)).as_u64())
                .context("--split size is too large for this platform")
        })
        .transpose()?;
    if let Some(size) = max_file_size {
        ensure!(
            size >= MIN_SPLIT_PART_BYTES,
            "The value for --split must be at least {MIN_SPLIT_PART_BYTES} bytes ({}).",
            ByteSize::b(MIN_SPLIT_PART_BYTES as u64)
        );
    }
    if !args.overwrite && archive.exists() {
        anyhow::bail!("{} already exists", archive.display());
    }
    log::info!("Create an archive: {}", archive.display());
    let mut files = args.file.files();
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
    let archive_path = current_dir.join(archive);
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
    let target_items = collect_items_from_paths(&files, &collect_options, &mut resolver)?
        .into_iter()
        .map(CollectedItem::Filesystem)
        .collect::<Vec<_>>();

    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)?;
    }
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
    let pathname_editor = PathnameEditor::new(
        args.strip_components,
        PathTransformers::new(args.substitutions, args.transforms),
        false,
    );
    let password = password.as_deref();
    let write_option = entry_option(args.compression, args.cipher, args.hash, password);
    let creation_context = CreationContext {
        write_option,
        keep_options,
        solid: args.solid,
        pathname_editor,
    };
    if let Some(size) = max_file_size {
        create_archive_with_split(
            &archive_path,
            creation_context,
            target_items,
            size,
            args.overwrite,
            &filter,
            &time_filters,
            password,
        )?;
    } else {
        create_archive_file(
            || utils::fs::file_create(&archive_path, args.overwrite),
            creation_context,
            target_items,
            &filter,
            &time_filters,
            password,
            false,
        )?;
    }
    log::info!(
        "Successfully created an archive in {}",
        DurationDisplay(start.elapsed())
    );
    Ok(())
}

pub(crate) struct CreationContext {
    pub(crate) write_option: WriteOptions,
    pub(crate) keep_options: KeepOptions,
    pub(crate) solid: bool,
    pub(crate) pathname_editor: PathnameEditor,
}

pub(crate) fn create_archive_file<W, F>(
    mut get_writer: F,
    CreationContext {
        write_option,
        keep_options,
        solid,
        pathname_editor,
    }: CreationContext,
    target_items: Vec<CollectedItem>,
    filter: &PathFilter<'_>,
    time_filters: &TimeFilters,
    password: Option<&[u8]>,
    verbose: bool,
) -> anyhow::Result<()>
where
    W: Write,
    F: FnMut() -> io::Result<W> + Send,
{
    let option = if solid {
        WriteOptions::store()
    } else {
        write_option.clone()
    };
    let create_options = CreateOptions {
        option,
        keep_options,
        pathname_editor,
    };
    let rx = spawn_entry_results(
        target_items,
        &create_options,
        filter,
        time_filters,
        password,
    );

    let file = get_writer()?;
    let buffered = io::BufWriter::with_capacity(64 * 1024, file);
    if solid {
        let mut writer = Archive::write_solid_header(buffered, write_option)?;
        drain_entry_results(rx, |entry| {
            if verbose {
                eprintln!("a {}", entry.name());
            }
            writer.add_entry(entry)
        })?;
        writer.finalize()?;
    } else {
        let mut writer = Archive::write_header(buffered)?;
        drain_entry_results(rx, |entry| {
            if verbose {
                eprintln!("a {}", entry.name());
            }
            writer.add_entry(entry)
        })?;
        writer.finalize()?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_archive_with_split(
    archive: &Path,
    CreationContext {
        write_option,
        keep_options,
        solid,
        pathname_editor,
    }: CreationContext,
    target_items: Vec<CollectedItem>,
    max_file_size: usize,
    overwrite: bool,
    filter: &PathFilter<'_>,
    time_filters: &TimeFilters,
    password: Option<&[u8]>,
) -> anyhow::Result<()> {
    let option = if solid {
        WriteOptions::store()
    } else {
        write_option.clone()
    };
    let create_options = CreateOptions {
        option,
        keep_options,
        pathname_editor,
    };
    let rx = spawn_entry_results(
        target_items,
        &create_options,
        filter,
        time_filters,
        password,
    );
    if solid {
        let mut entries_builder = SolidEntryBuilder::new(write_option)?;
        drain_entry_results(rx, |entry| entries_builder.add_entry(entry))?;
        let entries = entries_builder.build();
        write_split_archive(archive, [entries].into_iter(), max_file_size, overwrite)?;
    } else {
        let entries = ReorderByIndex::new(rx.into_iter()).flat_map(EntryResult::into_entries);
        write_split_archive(
            archive,
            entries.filter_map(Result::transpose),
            max_file_size,
            overwrite,
        )?;
    }
    Ok(())
}
