use crate::{
    cli::{
        CipherAlgorithmArgs, ColorChoice, CompressionAlgorithmArgs, DateTime, HashAlgorithmArgs,
        NameIdPair, PasswordArgs,
    },
    command::{
        Command,
        append::{open_archive_then_seek_to_end, run_append_archive},
        ask_password, check_password,
        core::{
            AclStrategy, CollectOptions, CreateOptions, KeepOptions, OwnerOptions, PathFilter,
            PathTransformers, PathnameEditor, PermissionStrategy, TimeFilterResolver, TimeOptions,
            TimestampStrategy, XattrStrategy, apply_chroot, collect_items_from_paths,
            collect_split_archives, entry_option,
            path_lock::PathLocks,
            re::{bsd::SubstitutionRule, gnu::TransformRule},
            read_paths,
        },
        create::{CreationContext, create_archive_file},
        extract::{OutputOption, OverwriteStrategy, run_extract_archive_reader},
        list::{Format, ListOptions, TimeField, TimeFormat},
    },
    utils::{self, GlobPatterns, VCS_FILES},
};
use clap::{ArgGroup, Args, ValueHint};
use pna::Archive;
use std::{env, io, path::PathBuf, sync::Arc, time::SystemTime};

#[derive(Args, Clone, Debug)]
#[clap(disable_help_flag = true)]
#[command(
    version,
    disable_version_flag = true,
    group(ArgGroup::new("keep-acl-flag").args(["keep_acl", "no_keep_acl"])),
    group(
        ArgGroup::new("from-input")
            .args(["files_from", "exclude_from"])
            .multiple(true)
    ),
    group(ArgGroup::new("null-requires").arg("null").requires("from-input")),
    group(ArgGroup::new("path-transform").args(["substitutions", "transforms"])),
    group(ArgGroup::new("owner-flag").args(["same_owner", "no_same_owner"])),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
    group(ArgGroup::new("recursive-flag").args(["recursive", "no_recursive"])),
    group(ArgGroup::new("keep-dir-flag").args(["keep_dir", "no_keep_dir"])),
    group(ArgGroup::new("keep-xattr-flag").args(["keep_xattr", "no_keep_xattr"])),
    group(ArgGroup::new("keep-timestamp-flag").args(["keep_timestamp", "no_keep_timestamp"])),
    group(ArgGroup::new("keep-permission-flag").args(["keep_permission", "no_keep_permission"])),
    group(ArgGroup::new("action-flags").args(["create", "extract", "list", "append"]).required(true)),
    group(ArgGroup::new("ctime-flag").args(["clamp_ctime"]).requires("ctime")),
    group(ArgGroup::new("mtime-flag").args(["clamp_mtime"]).requires("mtime")),
    group(ArgGroup::new("atime-flag").args(["clamp_atime"]).requires("atime")),
    group(ArgGroup::new("safe-writes-flag").args(["safe_writes", "no_safe_writes"])),
    group(
        ArgGroup::new("overwrite-flag")
            .args(["overwrite", "no_overwrite", "keep_newer_files", "keep_old_files"])
    ),
    group(ArgGroup::new("ctime-older-than-source").args(["older_ctime", "older_ctime_than"])),
    group(ArgGroup::new("ctime-newer-than-source").args(["newer_ctime", "newer_ctime_than"])),
    group(ArgGroup::new("mtime-older-than-source").args(["older_mtime", "older_mtime_than"])),
    group(ArgGroup::new("mtime-newer-than-source").args(["newer_mtime", "newer_mtime_than"])),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission", "no_keep_permission"]).requires("unstable")),
))]
pub(crate) struct StdioCommand {
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
    #[arg(short = 'c', long, help = "Create archive")]
    create: bool,
    #[arg(short = 'x', long, help = "Extract archive")]
    extract: bool,
    #[arg(short = 't', long, help = "List files in archive")]
    list: bool,
    #[arg(short = 'r', long, help = "Append files to archive")]
    append: bool,
    #[arg(
        long,
        visible_alias = "recursion",
        help = "Add directories to the archive recursively",
        default_value_t = true
    )]
    recursive: bool,
    #[arg(
        short = 'n',
        long,
        visible_aliases = ["norecurse", "no-recursion"],
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
    #[arg(
        long,
        requires = "unstable",
        help = "Skip extracting files if a newer version already exists (unstable)"
    )]
    keep_newer_files: bool,
    #[arg(
        short = 'U',
        long = "unlink-first",
        visible_alias = "unlink",
        requires_all = ["extract", "unstable"],
        help = "Unlink files before creating them; also removes intervening directory symlinks (extract mode only) (unstable)"
    )]
    unlink_first: bool,
    #[arg(
        short = 'k',
        long,
        requires = "unstable",
        help = "Skip extracting files if they already exist (unstable)"
    )]
    keep_old_files: bool,
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
        short = 'm',
        long,
        visible_aliases = ["no-preserve-timestamps", "modification_time"],
        help = "Do not archive timestamp of files. This is the inverse option of --preserve-timestamps"
    )]
    no_keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Preserve file permissions (unstable on Windows)"
    )]
    keep_permission: bool,
    #[arg(
        long,
        visible_aliases = ["no-preserve-permissions", "no-permissions"],
        help = "Do not archive permissions of files. This is the inverse option of --preserve-permissions"
    )]
    no_keep_permission: bool,
    #[arg(
        long,
        visible_aliases = ["preserve-xattrs", "xattrs"],
        help = "Preserve extended attributes"
    )]
    keep_xattr: bool,
    #[arg(
        long,
        visible_aliases = ["no-preserve-xattrs", "no-xattrs"],
        help = "Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs"
    )]
    no_keep_xattr: bool,
    #[arg(
        long,
        visible_aliases = ["preserve-acls", "acls"],
        requires = "unstable",
        help = "Preserve ACLs (unstable)"
    )]
    keep_acl: bool,
    #[arg(
        long,
        visible_aliases = ["no-preserve-acls", "no-acls"],
        requires = "unstable",
        help = "Do not archive ACLs. This is the inverse option of --keep-acl (unstable)"
    )]
    no_keep_acl: bool,
    #[arg(
        long,
        help = "Compress multiple files together for better compression ratio"
    )]
    solid: bool,
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) hash: HashAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
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
        short = 'X',
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
    #[arg(short = 'L', long, visible_aliases = ["dereference"], help = "Follow symbolic links")]
    follow_links: bool,
    #[arg(
        short = 'H',
        long,
        requires = "unstable",
        help = "Follow symbolic links named on the command line (unstable)"
    )]
    follow_command_links: bool,
    #[arg(long, value_name = "DIRECTORY", help = "Output directory of extracted files", value_hint = ValueHint::DirPath)]
    out_dir: Option<PathBuf>,
    #[arg(
        long,
        value_name = "N",
        help = "Remove the specified number of leading path elements. Path names with fewer elements will be silently skipped"
    )]
    strip_components: Option<usize>,
    #[arg(
        long,
        value_name = "NAME[:ID]",
        requires = "unstable",
        conflicts_with_all = ["uname", "uid", "numeric_owner"],
        help = "Use the provided owner, if uid is not provided, name can be either a user name or numeric id. See the --uname option for details (unstable)."
    )]
    owner: Option<NameIdPair>,
    #[arg(
        long,
        value_name = "NAME",
        help = "On create, archiving user to the entries from given name. On extract, restore user from given name"
    )]
    uname: Option<String>,
    #[arg(
        long,
        value_name = "NAME",
        help = "On create, archiving group to the entries from given name. On extract, restore group from given name"
    )]
    gname: Option<String>,
    #[arg(
        long,
        value_name = "ID",
        help = "On create, this overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id. On extract, this overrides the user id in the archive; the user name in the archive will be ignored"
    )]
    uid: Option<u32>,
    #[arg(
        long,
        value_name = "ID",
        help = "On create, this overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id. On extract, this overrides the group id in the archive; the group name in the archive will be ignored"
    )]
    gid: Option<u32>,
    #[arg(
        long,
        value_name = "NAME[:ID]",
        requires = "unstable",
        conflicts_with_all = ["gname", "gid", "numeric_owner"],
        help = "Use the provided group, if gid is not provided, name can be either a group name or numeric id. See the --gname option for details (unstable)."
    )]
    group: Option<NameIdPair>,
    #[arg(
        long,
        help = "This is equivalent to --uname \"\" --gname \"\". On create, it causes user and group names to not be stored in the archive. On extract, it causes user and group names in the archive to be ignored in favor of the numeric user and group ids."
    )]
    numeric_owner: bool,
    #[arg(long, value_name = "DATETIME", help = "Overrides the creation time")]
    ctime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the creation time of the entries to the specified time by --ctime"
    )]
    clamp_ctime: bool,
    #[arg(long, value_name = "DATETIME", help = "Overrides the access time")]
    atime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the access time of the entries to the specified time by --atime"
    )]
    clamp_atime: bool,
    #[arg(
        long,
        value_name = "DATETIME",
        help = "Overrides the modification time"
    )]
    mtime: Option<DateTime>,
    #[arg(
        long,
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
        value_name = "file",
        requires = "unstable",
        help = "Only include files and directories newer than the specified file (unstable). This compares ctime entries."
    )]
    newer_ctime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "file",
        requires = "unstable",
        visible_alias = "newer-than",
        help = "Only include files and directories newer than the specified file (unstable). This compares mtime entries."
    )]
    newer_mtime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "file",
        requires = "unstable",
        help = "Only include files and directories older than the specified file (unstable). This compares ctime entries."
    )]
    older_ctime_than: Option<PathBuf>,
    #[arg(
        long,
        value_name = "file",
        requires = "unstable",
        visible_alias = "older-than",
        help = "Only include files and directories older than the specified file (unstable). This compares mtime entries."
    )]
    older_mtime_than: Option<PathBuf>,
    #[arg(
        short = 'T',
        visible_short_aliases = ['I'],
        long,
        value_name = "FILE",
        requires = "unstable",
        help = "Read archiving files from given path (unstable)",
        value_hint = ValueHint::FilePath
    )]
    files_from: Option<PathBuf>,
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
        help = "Change directory before adding the following files",
        value_hint = ValueHint::DirPath
    )]
    working_dir: Option<PathBuf>,
    #[arg(
        short = 'O',
        long = "to-stdout",
        requires = "unstable",
        help = "Write extracted file data to standard output instead of the file system"
    )]
    to_stdout: bool,
    #[arg(
        long,
        help = "Allow extracting symbolic links and hard links that contain root or parent paths"
    )]
    allow_unsafe_links: bool,
    #[arg(
        long,
        requires = "extract",
        help = "chroot() to the current directory after processing any --cd options and before extracting any files (requires root privileges)"
    )]
    chroot: bool,
    #[arg(
        short = 'P',
        long = "absolute-paths",
        requires = "unstable",
        help = "Do not strip leading '/' or '..' from member names and link targets (unstable)"
    )]
    absolute_paths: bool,
    #[arg(
        short,
        long,
        help = "Read the archive from or write the archive to the specified file. The filename can be - for standard input or standard output."
    )]
    file: Option<String>,
    #[arg(help = "Files or patterns")]
    files: Vec<String>,
    #[arg(
        long,
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
    #[arg(short, help = "Verbose")]
    verbose: bool,
    #[arg(short = 'B', long, hide = true)]
    read_full_blocks: bool,
    #[arg(long, hide = true)]
    format: Option<String>,
    #[arg(long, hide = true)]
    posix: bool,
    #[arg(long, hide = true)]
    ignore_zeros: bool,
    #[arg(long, hide = true)]
    safe_writes: bool,
    #[arg(long, hide = true)]
    no_safe_writes: bool,
    #[arg(short = 'a', long = "auto-compress", hide = true)]
    auto_compress: bool,
    #[arg(long, action = clap::ArgAction::Version, help = "Print version")]
    version: (),
    #[arg(long, action = clap::ArgAction::Help, help = "Print help")]
    help: (),
}

impl Command for StdioCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        run_stdio(self)
    }
}

fn run_stdio(args: StdioCommand) -> anyhow::Result<()> {
    if let Some(format) = &args.format {
        log::warn!("Option '--format {format}' is accepted for compatibility but will be ignored.");
    }
    if args.posix {
        log::warn!("Option '--posix' is accepted for compatibility but will be ignored.");
    }
    if args.read_full_blocks {
        log::warn!(
            "Option '--read-full-blocks' is accepted for compatibility but will be ignored."
        );
    }
    if args.ignore_zeros {
        log::warn!("Option '--ignore-zeros' is accepted for compatibility but will be ignored.");
    }
    if args.safe_writes {
        log::warn!("Option '--safe-writes' is accepted for compatibility but will be ignored.");
    }
    if args.no_safe_writes {
        log::warn!("Option '--no-safe-writes' is accepted for compatibility but will be ignored.");
    }
    if args.auto_compress {
        log::warn!("Option '--auto-compress' is accepted for compatibility but will be ignored.");
    }
    if args.create {
        run_create_archive(args)
    } else if args.extract {
        run_extract_archive(args)
    } else if args.list {
        run_list_archive(args)
    } else if args.append {
        run_append(args)
    } else {
        unreachable!()
    }
}

fn run_create_archive(args: StdioCommand) -> anyhow::Result<()> {
    let current_dir = env::current_dir()?;
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    // NOTE: "-" will use stdout
    let mut file = args.file;
    file.take_if(|it| it == "-");
    let archive_file = file.take().map(|p| current_dir.join(p));
    let mut files = args.files;
    if let Some(path) = args.files_from {
        files.extend(read_paths(path, args.null)?);
    }

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
    let target_items = collect_items_from_paths(&files, &collect_options)?;

    let password = password.as_deref();
    let cli_option = entry_option(args.compression, args.cipher, args.hash, password);
    let keep_options = KeepOptions {
        timestamp_strategy: TimestampStrategy::from_flags(
            args.keep_timestamp,
            args.no_keep_timestamp,
            TimestampStrategy::Always,
        ),
        permission_strategy: PermissionStrategy::from_flags(
            args.keep_permission,
            args.no_keep_permission,
        ),
        xattr_strategy: XattrStrategy::from_flags(args.keep_xattr, args.no_keep_xattr),
        acl_strategy: AclStrategy::from_flags(args.keep_acl, args.no_keep_acl),
    };
    let owner_options = resolve_owner_options(
        args.owner,
        args.group,
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
    let creation_context = CreationContext {
        write_option: cli_option,
        keep_options,
        owner_options,
        time_options,
        solid: args.solid,
        pathname_editor: PathnameEditor::new(
            args.strip_components,
            PathTransformers::new(args.substitutions, args.transforms),
            args.absolute_paths,
        ),
    };
    if let Some(file) = archive_file {
        create_archive_file(
            || utils::fs::file_create(&file, args.overwrite),
            creation_context,
            target_items,
        )
    } else {
        create_archive_file(|| Ok(io::stdout().lock()), creation_context, target_items)
    }
}

fn run_extract_archive(args: StdioCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;

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

    let overwrite_strategy = OverwriteStrategy::from_flags(
        args.overwrite,
        args.no_overwrite,
        args.keep_newer_files,
        args.keep_old_files,
        OverwriteStrategy::Always,
    );
    let out_option = OutputOption {
        overwrite_strategy,
        allow_unsafe_links: args.allow_unsafe_links,
        out_dir: args.out_dir,
        to_stdout: args.to_stdout,
        filter,
        keep_options: KeepOptions {
            timestamp_strategy: TimestampStrategy::from_flags(
                args.keep_timestamp,
                args.no_keep_timestamp,
                TimestampStrategy::Always,
            ),
            permission_strategy: PermissionStrategy::from_flags(
                args.keep_permission,
                args.no_keep_permission,
            ),
            xattr_strategy: XattrStrategy::from_flags(args.keep_xattr, args.no_keep_xattr),
            acl_strategy: AclStrategy::from_flags(args.keep_acl, args.no_keep_acl),
        },
        owner_options: resolve_owner_options(
            args.owner,
            args.group,
            args.uname,
            args.gname,
            args.uid,
            args.gid,
            args.numeric_owner,
        ),
        same_owner: !args.no_same_owner,
        pathname_editor: PathnameEditor::new(
            args.strip_components,
            PathTransformers::new(args.substitutions, args.transforms),
            args.absolute_paths,
        ),
        path_locks: Arc::new(PathLocks::default()),
        unlink_first: args.unlink_first,
    };
    let mut files = args.files;
    if let Some(path) = &args.files_from {
        files.extend(read_paths(path, args.null)?);
    }
    // NOTE: "-" will use stdin
    let mut file = args.file;
    file.take_if(|it| it == "-");
    let archives = if let Some(path) = &file {
        Some(collect_split_archives(path)?)
    } else {
        None
    };
    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
    }
    apply_chroot(args.chroot)?;
    if let Some(archives) = archives {
        run_extract_archive_reader(
            archives
                .into_iter()
                .map(|it| io::BufReader::with_capacity(64 * 1024, it)),
            files,
            || password.as_deref(),
            out_option,
        )
    } else {
        run_extract_archive_reader(
            std::iter::repeat_with(|| io::stdin().lock()),
            files,
            || password.as_deref(),
            out_option,
        )
    }
}

fn run_list_archive(args: StdioCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let list_options = ListOptions {
        long: false,
        header: false,
        solid: true,
        show_xattr: false,
        show_acl: false,
        show_private: false,
        time_format: TimeFormat::Auto(SystemTime::now()),
        time_field: TimeField::default(),
        numeric_owner: args.numeric_owner,
        hide_control_chars: false,
        classify: true,
        format: Some(if args.verbose {
            Format::BsdTar
        } else {
            Format::Line
        }),
        out_to_stderr: args.to_stdout,
        color: ColorChoice::Auto,
    };
    let files_globs = GlobPatterns::new(args.files.iter().map(|it| it.as_str()))?;

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
    // NOTE: "-" will use stdout
    let mut file = args.file;
    file.take_if(|it| it == "-");
    if let Some(path) = &file {
        let archives = collect_split_archives(path)?;
        crate::command::list::run_list_archive(
            archives
                .into_iter()
                .map(|it| io::BufReader::with_capacity(64 * 1024, it)),
            password.as_deref(),
            files_globs,
            filter,
            list_options,
        )
    } else {
        crate::command::list::run_list_archive(
            std::iter::repeat_with(|| io::stdin().lock()),
            password.as_deref(),
            files_globs,
            filter,
            list_options,
        )
    }
}

fn run_append(args: StdioCommand) -> anyhow::Result<()> {
    let current_dir = env::current_dir()?;
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let password = password.as_deref();
    let option = entry_option(args.compression, args.cipher, args.hash, password);
    let keep_options = KeepOptions {
        timestamp_strategy: TimestampStrategy::from_flags(
            args.keep_timestamp,
            args.no_keep_timestamp,
            TimestampStrategy::Always,
        ),
        permission_strategy: PermissionStrategy::from_flags(
            args.keep_permission,
            args.no_keep_permission,
        ),
        xattr_strategy: XattrStrategy::from_flags(args.keep_xattr, args.no_keep_xattr),
        acl_strategy: AclStrategy::from_flags(args.keep_acl, args.no_keep_acl),
    };
    let owner_options = resolve_owner_options(
        args.owner,
        args.group,
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
    let create_options = CreateOptions {
        option,
        keep_options,
        owner_options,
        time_options,
        pathname_editor: PathnameEditor::new(
            args.strip_components,
            PathTransformers::new(args.substitutions, args.transforms),
            args.absolute_paths,
        ),
    };

    // NOTE: "-" will use stdin/out
    let mut file = args.file;
    file.take_if(|it| it == "-");
    let archive_path = file.take().map(|p| current_dir.join(p));
    let mut files = args.files;
    if let Some(path) = args.files_from {
        files.extend(read_paths(path, args.null)?);
    }

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
    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
    }
    let collect_options = CollectOptions {
        recursive: args.recursive,
        keep_dir: args.keep_dir,
        gitignore: args.gitignore,
        nodump: args.nodump,
        follow_links: args.follow_links,
        follow_command_links: args.follow_command_links,
        one_file_system: args.one_file_system,
        filter: &filter,
        time_filters: &time_filters,
    };
    if let Some(file) = &archive_path {
        let archive = open_archive_then_seek_to_end(file)?;
        let target_items = collect_items_from_paths(&files, &collect_options)?;
        run_append_archive(&create_options, archive, target_items)
    } else {
        let target_items = collect_items_from_paths(&files, &collect_options)?;
        let mut output_archive = Archive::write_header(io::stdout().lock())?;
        {
            let mut input_archive = Archive::read_header(io::stdin().lock())?;
            for entry in input_archive.raw_entries() {
                output_archive.add_entry(entry?)?;
            }
        }
        run_append_archive(&create_options, output_archive, target_items)
    }
}

fn resolve_owner_options(
    owner: Option<NameIdPair>,
    group: Option<NameIdPair>,
    uname: Option<String>,
    gname: Option<String>,
    uid: Option<u32>,
    gid: Option<u32>,
    numeric_owner: bool,
) -> OwnerOptions {
    let (uname, uid) = resolve_name_id(owner, uname, uid);
    let (gname, gid) = resolve_name_id(group, gname, gid);
    OwnerOptions::new(uname, gname, uid, gid, numeric_owner)
}

fn resolve_name_id(
    spec: Option<NameIdPair>,
    name: Option<String>,
    id: Option<u32>,
) -> (Option<String>, Option<u32>) {
    match spec {
        Some(spec) => (spec.name, spec.id),
        None => (name, id),
    }
}
