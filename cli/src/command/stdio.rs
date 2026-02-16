use crate::{
    cli::{
        CipherAlgorithmArgs, ColorChoice, DateTime, DeflateLevel, GlobalContext, HashAlgorithmArgs,
        MissingTimePolicy, NameIdPair, PasswordArgs, XzLevel, ZstdLevel,
    },
    command::{
        Command,
        append::{open_archive_then_seek_to_end, run_append_archive},
        ask_password, check_password,
        core::{
            AclStrategy, CollectOptions, CreateOptions, FflagsStrategy, ItemSource, KeepOptions,
            MacMetadataStrategy, ModeStrategy, OwnerOptions, OwnerStrategy, PathFilter,
            PathTransformers, PathnameEditor, SplitArchiveReader, TimeFilterResolver,
            TimestampStrategyResolver, TransformStrategyUnSolid, Umask, XattrStrategy,
            apply_chroot, collect_items_from_paths, collect_items_from_sources,
            collect_split_archives,
            path_lock::OrderedPathLocks,
            re::{bsd::SubstitutionRule, gnu::TransformRule},
            read_paths, validate_no_duplicate_stdin,
        },
        create::{CreationContext, create_archive_file},
        extract::{OutputOption, OverwriteStrategy, run_extract_archive_reader},
        list::{Format, ListOptions, TimeField, TimeFormat},
        update::run_update_archive,
    },
    utils::{
        self, BsdGlobMatcher, PathPartExt, VCS_FILES, env::NamedTempFile, fs::HardlinkResolver,
    },
};
use clap::{ArgGroup, Args, Parser, ValueHint};
use pna::Archive;
use std::{env, io, path::PathBuf, sync::Arc, time::SystemTime};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("stdio_compression_method").args(["store", "deflate", "zstd", "xz"])))]
struct CompressionAlgorithmArgs {
    #[arg(long, help = "No compression")]
    store: bool,
    #[arg(
        long,
        visible_alias = "zlib",
        value_name = "level",
        help = "Use deflate for compression [possible level: 1-9, min, max]"
    )]
    deflate: Option<Option<DeflateLevel>>,
    #[arg(
        long,
        value_name = "level",
        help = "Use zstd for compression [possible level: 1-21, min, max]"
    )]
    zstd: Option<Option<ZstdLevel>>,
    #[arg(
        short = 'J',
        long,
        value_name = "level",
        help = "Use xz for compression [possible level: 0-9, min, max]"
    )]
    xz: Option<Option<XzLevel>>,
}

impl CompressionAlgorithmArgs {
    fn algorithm(
        &self,
        options: Option<&crate::cli::ArchiveOptions>,
    ) -> (pna::Compression, Option<pna::CompressionLevel>) {
        let (compression, flag_level, module_level) = if self.store {
            (pna::Compression::No, None, None)
        } else if let Some(level) = self.xz {
            (
                pna::Compression::XZ,
                level.map(Into::into),
                options.and_then(|o| o.xz_compression_level.map(Into::into)),
            )
        } else if let Some(level) = self.zstd {
            (
                pna::Compression::ZStandard,
                level.map(Into::into),
                options.and_then(|o| o.zstd_compression_level.map(Into::into)),
            )
        } else if let Some(level) = self.deflate {
            (
                pna::Compression::Deflate,
                level.map(Into::into),
                options.and_then(|o| o.deflate_compression_level.map(Into::into)),
            )
        } else {
            (pna::Compression::ZStandard, None, None)
        };

        if flag_level.is_some() {
            log::warn!(
                "compression level in flags is deprecated, use `--options='compression-level=N'` instead"
            );
        }

        let global_level = options.and_then(|o| o.compression_level);
        let level = module_level.or(global_level).or(flag_level);

        (compression, level)
    }
}

#[derive(Args, Clone, Debug)]
#[clap(disable_help_flag = true)]
#[command(
    display_name = "bsdtar",
    version,
    // Reference bsdtar version this implementation targets
    long_version = concat!("3.7.4 - portable-network-archive ", env!("CARGO_PKG_VERSION")),
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
    group(ArgGroup::new("action-flags").args(["create", "extract", "list", "append", "update"]).required(true)),
    group(ArgGroup::new("safe-writes-flag").args(["safe_writes", "no_safe_writes"])),
    group(ArgGroup::new("unsafe-links-flag").args(["allow_unsafe_links", "no_allow_unsafe_links"])),
    group(
        ArgGroup::new("overwrite-flag")
            .args(["overwrite", "no_overwrite", "keep_newer_files", "keep_old_files"])
    ),
    group(ArgGroup::new("ctime-older-than-source").args(["older_ctime", "older_ctime_than"])),
    group(ArgGroup::new("ctime-newer-than-source").args(["newer_ctime", "newer_ctime_than"])),
    group(ArgGroup::new("mtime-older-than-source").args(["older_mtime", "older_mtime_than"])),
    group(ArgGroup::new("mtime-newer-than-source").args(["newer_mtime", "newer_mtime_than"])),
    group(ArgGroup::new("keep-fflags-flag").args(["keep_fflags", "no_keep_fflags"])),
    group(ArgGroup::new("mac-metadata-flag").args(["mac_metadata", "no_mac_metadata"])),
)]
pub(crate) struct StdioCommand {
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
    #[arg(short = 'c', long, help = "Create archive")]
    create: bool,
    #[arg(short = 'x', long, help = "Extract archive")]
    extract: bool,
    #[arg(short = 't', long, help = "List files in archive")]
    list: bool,
    #[arg(
        short = 'q',
        long,
        help = "Performance optimization for list/extract: stop after the first match for each operand and ignore later duplicates"
    )]
    fast_read: bool,
    #[arg(short = 'r', long, help = "Append files to archive")]
    append: bool,
    #[arg(short = 'u', long, help = "Update archive with newer files")]
    update: bool,
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
        help_heading = "Unstable Options",
        help = "Skip extracting files if a newer version already exists"
    )]
    keep_newer_files: bool,
    #[arg(
        short = 'U',
        long = "unlink-first",
        visible_alias = "unlink",
        requires_all = ["extract", "unstable"],
        help_heading = "Unstable Options",
        help = "Unlink files before creating them; also removes intervening directory symlinks (extract mode only)"
    )]
    unlink_first: bool,
    #[arg(
        short = 'k',
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Skip extracting files if they already exist"
    )]
    keep_old_files: bool,
    #[arg(long, help = "Include directories in archive (default)")]
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
        visible_aliases = ["no-preserve-permissions", "no-permissions"],
        help = "Do not store file permissions (mode bits) in the archive"
    )]
    no_same_permissions: bool,
    #[arg(
        short = 'p',
        long,
        visible_alias = "preserve-permissions",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Restore file permissions (mode, ACLs, xattrs, fflags, mac-metadata, but NOT ownership) (extract only)"
    )]
    same_permissions: bool,
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
        help_heading = "Unstable Options",
        help = "Preserve ACLs"
    )]
    keep_acl: bool,
    #[arg(
        long,
        visible_aliases = ["no-preserve-acls", "no-acls"],
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Do not archive ACLs. This is the inverse option of --keep-acl"
    )]
    no_keep_acl: bool,
    #[arg(
        long,
        visible_aliases = ["preserve-fflags", "fflags"],
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Archiving the file flags of the files"
    )]
    keep_fflags: bool,
    #[arg(
        long,
        visible_aliases = ["no-preserve-fflags", "no-fflags"],
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Do not archive file flags of files. This is the inverse option of --keep-fflags"
    )]
    no_keep_fflags: bool,
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Archive and extract Mac metadata (extended attributes and ACLs)"
    )]
    mac_metadata: bool,
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Do not archive or extract Mac metadata. This is the inverse option of --mac-metadata"
    )]
    no_mac_metadata: bool,
    #[arg(
        long,
        help = "Compress multiple files together for better compression ratio"
    )]
    solid: bool,
    #[command(flatten)]
    compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) hash: HashAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[arg(
        long,
        value_name = "OPTIONS",
        help = "Comma-separated list of options. Format: key=value or module:key=value. Supported: compression-level. Modules: deflate, zstd, xz"
    )]
    options: Option<crate::cli::ArchiveOptions>,
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
        short = 'X',
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
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Ignore files from .gitignore"
    )]
    gitignore: bool,
    #[arg(short = 'L', long, visible_aliases = ["dereference"], help = "Follow symbolic links")]
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
        short = 'l',
        long,
        visible_alias = "check-links",
        requires = "create",
        help = "Warn if not all links to each file are archived (create mode)"
    )]
    check_links: bool,
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
        help_heading = "Unstable Options",
        help = "Use the provided owner, if uid is not provided, name can be either a user name or numeric id. See the --uname option for details."
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
        help_heading = "Unstable Options",
        help = "Use the provided group, if gid is not provided, name can be either a group name or numeric id. See the --gname option for details."
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
        visible_alias = "newer-than",
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
        visible_alias = "older-than",
        help_heading = "Unstable Options",
        help = "Only include files and directories older than the specified file. This compares mtime entries."
    )]
    older_mtime_than: Option<PathBuf>,
    #[arg(
        short = 'T',
        visible_short_aliases = ['I'],
        long,
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Read archiving files from given path",
        value_hint = ValueHint::FilePath
    )]
    files_from: Option<PathBuf>,
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
        help = "Change directory before adding the following files",
        value_hint = ValueHint::DirPath
    )]
    working_dir: Option<PathBuf>,
    #[arg(
        short = 'O',
        long = "to-stdout",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Write extracted file data to standard output instead of the file system"
    )]
    to_stdout: bool,
    #[arg(
        long,
        help = "Allow extracting symbolic links and hard links that contain root or parent paths (default)"
    )]
    allow_unsafe_links: bool,
    #[arg(
        long,
        help = "Do not allow extracting symbolic links and hard links that contain root or parent paths"
    )]
    no_allow_unsafe_links: bool,
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
        help_heading = "Unstable Options",
        help = "Do not strip leading '/' or '..' from member names and link targets"
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
    #[arg(
        short = 'b',
        long = "block-size",
        value_name = "blocksize",
        hide = true
    )]
    block_size: Option<usize>,
    #[arg(long, action = clap::ArgAction::Version, help = "Print version")]
    version: (),
    #[arg(short = 'h', long, action = clap::ArgAction::Help, help = "Print help")]
    help: (),
}

impl Command for StdioCommand {
    #[inline]
    fn execute(self, ctx: &GlobalContext) -> anyhow::Result<()> {
        run_stdio(ctx, self)
    }
}

#[hooq::hooq(anyhow)]
fn run_stdio(ctx: &GlobalContext, args: StdioCommand) -> anyhow::Result<()> {
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
    if args.auto_compress {
        log::warn!("Option '--auto-compress' is accepted for compatibility but will be ignored.");
    }
    if let Some(block_size) = &args.block_size {
        log::warn!(
            "Option '--block-size {block_size}' is accepted for compatibility but will be ignored."
        );
    }
    if args.create {
        run_create_archive(args)
    } else if args.extract {
        run_extract_archive(ctx, args)
    } else if args.list {
        run_list_archive(args)
    } else if args.append {
        run_append(args)
    } else if args.update {
        run_update(args)
    } else {
        unreachable!()
    }
}

fn build_write_options(
    compression: &CompressionAlgorithmArgs,
    cipher: &CipherAlgorithmArgs,
    hash: &HashAlgorithmArgs,
    options: Option<&crate::cli::ArchiveOptions>,
    password: Option<&[u8]>,
) -> pna::WriteOptions {
    let (algorithm, level) = compression.algorithm(options);
    let mut option_builder = pna::WriteOptions::builder();
    option_builder
        .compression(algorithm)
        .compression_level(level.unwrap_or_default())
        .encryption(if password.is_some() {
            cipher.algorithm()
        } else {
            pna::Encryption::No
        })
        .cipher_mode(cipher.mode())
        .hash_algorithm(hash.algorithm())
        .password(password);
    option_builder.build()
}

/// Resolves permission strategies for stdio creation operations (create/append/update).
/// Creation defaults: store mode + owner by default (bsdtar behavior).
struct CreationPermissionStrategyResolver {
    no_same_permissions: bool,
    no_same_owner: bool,
    numeric_owner: bool,
    uname: Option<String>,
    gname: Option<String>,
    uid: Option<u32>,
    gid: Option<u32>,
}

impl CreationPermissionStrategyResolver {
    fn resolve(self) -> (ModeStrategy, OwnerStrategy) {
        let mode_strategy = if self.no_same_permissions {
            ModeStrategy::Never
        } else {
            ModeStrategy::Preserve
        };
        let owner_strategy = if self.no_same_owner {
            OwnerStrategy::Never
        } else {
            OwnerStrategy::Preserve {
                options: OwnerOptions {
                    uname: if self.numeric_owner {
                        Some(String::new())
                    } else {
                        self.uname
                    },
                    gname: if self.numeric_owner {
                        Some(String::new())
                    } else {
                        self.gname
                    },
                    uid: self.uid,
                    gid: self.gid,
                },
            }
        };
        (mode_strategy, owner_strategy)
    }
}

/// Resolves permission strategies for stdio extraction operations.
/// Extraction defaults: root preserves exact permissions, non-root applies umask (bsdtar behavior).
/// -p/--same-permissions enables mode + ACL + xattr + fflags + mac-metadata (but NOT owner)
/// Flag priority: --no-same-permissions > -p > individual flags; individual --no-* always wins
struct ExtractionPermissionStrategyResolver {
    same_permissions: bool,
    no_same_permissions: bool,
    same_owner: bool,
    numeric_owner: bool,
    uname: Option<String>,
    umask: Umask,
    is_root: bool,
    gname: Option<String>,
    uid: Option<u32>,
    gid: Option<u32>,
    keep_xattr: bool,
    keep_acl: bool,
    keep_fflags: bool,
    mac_metadata: bool,
    no_keep_xattr: bool,
    no_keep_acl: bool,
    no_keep_fflags: bool,
    no_mac_metadata: bool,
}

type ExtractionPermissionStrategies = (
    ModeStrategy,
    OwnerStrategy,
    XattrStrategy,
    AclStrategy,
    FflagsStrategy,
    MacMetadataStrategy,
);

impl ExtractionPermissionStrategyResolver {
    fn resolve(self) -> ExtractionPermissionStrategies {
        // bsdtar behavior: root defaults to -p (Preserve), non-root defaults to Masked
        let default_mode_strategy = if self.is_root {
            ModeStrategy::Preserve
        } else {
            ModeStrategy::Masked(self.umask)
        };

        let mode_strategy = if self.no_same_permissions {
            ModeStrategy::Masked(self.umask)
        } else if self.same_permissions {
            ModeStrategy::Preserve
        } else {
            default_mode_strategy
        };

        let owner_strategy = if self.same_owner {
            OwnerStrategy::Preserve {
                options: OwnerOptions {
                    uname: if self.numeric_owner {
                        Some(String::new())
                    } else {
                        self.uname
                    },
                    gname: if self.numeric_owner {
                        Some(String::new())
                    } else {
                        self.gname
                    },
                    uid: self.uid,
                    gid: self.gid,
                },
            }
        } else {
            OwnerStrategy::Never
        };

        // -p enables these unless --no-same-permissions is set; individual --no-* always wins
        let p_enables = !self.no_same_permissions && self.same_permissions;

        (
            mode_strategy,
            owner_strategy,
            XattrStrategy::from_flags(self.keep_xattr || p_enables, self.no_keep_xattr),
            AclStrategy::from_flags(self.keep_acl || p_enables, self.no_keep_acl),
            FflagsStrategy::from_flags(self.keep_fflags || p_enables, self.no_keep_fflags),
            MacMetadataStrategy::from_flags(self.mac_metadata || p_enables, self.no_mac_metadata),
        )
    }
}

#[hooq::hooq(anyhow)]
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
    if files.is_empty() {
        anyhow::bail!("create mode requires at least one input path or @archive source");
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
    // Parse sources AFTER changing directory so @archive paths are affected by -C
    let sources = ItemSource::parse_many(&files);
    validate_no_duplicate_stdin(&sources)?;
    let collect_options = CollectOptions {
        recursive: !args.no_recursive,
        keep_dir: !args.no_keep_dir,
        gitignore: args.gitignore,
        nodump: args.nodump,
        follow_links: args.follow_links,
        follow_command_links: args.follow_command_links,
        one_file_system: args.one_file_system,
        filter: &filter,
        time_filters: &time_filters,
    };
    let mut resolver = HardlinkResolver::new(collect_options.follow_links);
    let target_items = collect_items_from_sources(sources, &collect_options, &mut resolver)?;
    if args.check_links {
        for (path, expected, archived) in resolver.incomplete_links() {
            log::warn!(
                "{}: file has {} links, only {} archived",
                path.display(),
                expected,
                archived
            );
        }
    }

    let password = password.as_deref();
    let cli_option = build_write_options(
        &args.compression,
        &args.cipher,
        &args.hash,
        args.options.as_ref(),
        password,
    );
    let (uname, uid) = resolve_name_id(args.owner, args.uname, args.uid);
    let (gname, gid) = resolve_name_id(args.group, args.gname, args.gid);
    let (mode_strategy, owner_strategy) = CreationPermissionStrategyResolver {
        no_same_permissions: args.no_same_permissions,
        no_same_owner: args.no_same_owner,
        numeric_owner: args.numeric_owner,
        uname,
        gname,
        uid,
        gid,
    }
    .resolve();
    let keep_options = KeepOptions {
        timestamp_strategy: TimestampStrategyResolver {
            keep_timestamp: args.keep_timestamp,
            no_keep_timestamp: args.no_keep_timestamp,
            default_preserve: true,
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
        fflags_strategy: FflagsStrategy::from_flags(args.keep_fflags, args.no_keep_fflags),
        mac_metadata_strategy: MacMetadataStrategy::from_flags(
            args.mac_metadata,
            args.no_mac_metadata,
        ),
    };
    let creation_context = CreationContext {
        write_option: cli_option,
        keep_options,
        solid: args.solid,
        pathname_editor: PathnameEditor::new(
            args.strip_components,
            PathTransformers::new(args.substitutions, args.transforms),
            args.absolute_paths,
        ),
    };
    if let Some(file) = archive_file {
        create_archive_file(
            || utils::fs::file_create(&file, !args.no_overwrite),
            creation_context,
            target_items,
            &filter,
            &time_filters,
            password,
            args.verbose,
        )
    } else {
        create_archive_file(
            || Ok(io::stdout().lock()),
            creation_context,
            target_items,
            &filter,
            &time_filters,
            password,
            args.verbose,
        )
    }
}

#[hooq::hooq(anyhow)]
fn run_extract_archive(ctx: &GlobalContext, args: StdioCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;

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

    let overwrite_strategy = OverwriteStrategy::from_flags(
        args.overwrite,
        args.no_overwrite,
        args.keep_newer_files,
        args.keep_old_files,
        OverwriteStrategy::Always,
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
        missing_ctime: MissingTimePolicy::Include,
        missing_mtime: MissingTimePolicy::Include,
    }
    .resolve()?;
    let (uname, uid) = resolve_name_id(args.owner, args.uname, args.uid);
    let (gname, gid) = resolve_name_id(args.group, args.gname, args.gid);
    let (
        mode_strategy,
        owner_strategy,
        xattr_strategy,
        acl_strategy,
        fflags_strategy,
        mac_metadata_strategy,
    ) = ExtractionPermissionStrategyResolver {
        same_permissions: args.same_permissions,
        no_same_permissions: args.no_same_permissions,
        same_owner: args.same_owner,
        numeric_owner: args.numeric_owner,
        uname,
        gname,
        uid,
        gid,
        umask: ctx.umask(),
        is_root: ctx.is_root(),
        keep_xattr: args.keep_xattr,
        keep_acl: args.keep_acl,
        keep_fflags: args.keep_fflags,
        mac_metadata: args.mac_metadata,
        no_keep_xattr: args.no_keep_xattr,
        no_keep_acl: args.no_keep_acl,
        no_keep_fflags: args.no_keep_fflags,
        no_mac_metadata: args.no_mac_metadata,
    }
    .resolve();
    let out_option = OutputOption {
        overwrite_strategy,
        allow_unsafe_links: !args.no_allow_unsafe_links,
        out_dir: args.out_dir,
        to_stdout: args.to_stdout,
        filter,
        keep_options: KeepOptions {
            timestamp_strategy: TimestampStrategyResolver {
                keep_timestamp: args.keep_timestamp,
                no_keep_timestamp: args.no_keep_timestamp,
                default_preserve: true,
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
            xattr_strategy,
            acl_strategy,
            fflags_strategy,
            mac_metadata_strategy,
        },
        pathname_editor: PathnameEditor::new(
            args.strip_components,
            PathTransformers::new(args.substitutions, args.transforms),
            args.absolute_paths,
        ),
        ordered_path_locks: Arc::new(OrderedPathLocks::default()),
        unlink_first: args.unlink_first,
        time_filters,
        safe_writes: args.safe_writes && !args.no_safe_writes,
        verbose: args.verbose,
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
            args.no_recursive,
            args.fast_read,
        )
    } else {
        run_extract_archive_reader(
            std::iter::repeat_with(|| io::stdin().lock()),
            files,
            || password.as_deref(),
            out_option,
            args.no_recursive,
            args.fast_read,
        )
    }
}

#[hooq::hooq(anyhow)]
fn run_list_archive(args: StdioCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
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

    let list_options = ListOptions {
        long: false,
        header: false,
        solid: true,
        show_xattr: false,
        show_acl: false,
        show_fflags: false,
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
        time_filters,
    };
    let files_globs = BsdGlobMatcher::new(args.files.iter().map(|it| it.as_str()))
        .with_no_recursive(args.no_recursive);

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
            args.fast_read,
        )
    } else {
        crate::command::list::run_list_archive(
            std::iter::repeat_with(|| io::stdin().lock()),
            password.as_deref(),
            files_globs,
            filter,
            list_options,
            args.fast_read,
        )
    }
}

#[hooq::hooq(anyhow)]
fn run_append(args: StdioCommand) -> anyhow::Result<()> {
    let current_dir = env::current_dir()?;
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let password = password.as_deref();
    let option = build_write_options(
        &args.compression,
        &args.cipher,
        &args.hash,
        args.options.as_ref(),
        password,
    );
    let (uname, uid) = resolve_name_id(args.owner, args.uname, args.uid);
    let (gname, gid) = resolve_name_id(args.group, args.gname, args.gid);
    let (mode_strategy, owner_strategy) = CreationPermissionStrategyResolver {
        no_same_permissions: args.no_same_permissions,
        no_same_owner: args.no_same_owner,
        numeric_owner: args.numeric_owner,
        uname,
        gname,
        uid,
        gid,
    }
    .resolve();
    let keep_options = KeepOptions {
        timestamp_strategy: TimestampStrategyResolver {
            keep_timestamp: args.keep_timestamp,
            no_keep_timestamp: args.no_keep_timestamp,
            default_preserve: true,
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
        fflags_strategy: FflagsStrategy::from_flags(args.keep_fflags, args.no_keep_fflags),
        mac_metadata_strategy: MacMetadataStrategy::from_flags(
            args.mac_metadata,
            args.no_mac_metadata,
        ),
    };
    let create_options = CreateOptions {
        option,
        keep_options,
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
    // Parse sources AFTER changing directory so @archive paths are affected by -C
    let sources = ItemSource::parse_many(&files);
    validate_no_duplicate_stdin(&sources)?;
    let collect_options = CollectOptions {
        recursive: args.recursive,
        keep_dir: !args.no_keep_dir,
        gitignore: args.gitignore,
        nodump: args.nodump,
        follow_links: args.follow_links,
        follow_command_links: args.follow_command_links,
        one_file_system: args.one_file_system,
        filter: &filter,
        time_filters: &time_filters,
    };
    let mut resolver = HardlinkResolver::new(collect_options.follow_links);
    if let Some(file) = &archive_path {
        let archive = open_archive_then_seek_to_end(file)?;
        let target_items = collect_items_from_sources(sources, &collect_options, &mut resolver)?;
        run_append_archive(
            &create_options,
            archive,
            target_items,
            &filter,
            &time_filters,
            password,
            args.verbose,
        )
    } else {
        let target_items = collect_items_from_sources(sources, &collect_options, &mut resolver)?;
        let mut output_archive = Archive::write_header(io::stdout().lock())?;
        {
            let mut input_archive = Archive::read_header(io::stdin().lock())?;
            for entry in input_archive.raw_entries() {
                output_archive.add_entry(entry?)?;
            }
        }
        run_append_archive(
            &create_options,
            output_archive,
            target_items,
            &filter,
            &time_filters,
            password,
            args.verbose,
        )
    }
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

fn run_update(args: StdioCommand) -> anyhow::Result<()> {
    let current_dir = env::current_dir()?;
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let password = password.as_deref();
    let option = build_write_options(
        &args.compression,
        &args.cipher,
        &args.hash,
        args.options.as_ref(),
        password,
    );
    let (uname, uid) = resolve_name_id(args.owner, args.uname, args.uid);
    let (gname, gid) = resolve_name_id(args.group, args.gname, args.gid);
    let (mode_strategy, owner_strategy) = CreationPermissionStrategyResolver {
        no_same_permissions: args.no_same_permissions,
        no_same_owner: args.no_same_owner,
        numeric_owner: args.numeric_owner,
        uname,
        gname,
        uid,
        gid,
    }
    .resolve();
    let keep_options = KeepOptions {
        timestamp_strategy: TimestampStrategyResolver {
            keep_timestamp: args.keep_timestamp,
            no_keep_timestamp: args.no_keep_timestamp,
            default_preserve: true,
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
        fflags_strategy: FflagsStrategy::from_flags(args.keep_fflags, args.no_keep_fflags),
        mac_metadata_strategy: MacMetadataStrategy::from_flags(
            args.mac_metadata,
            args.no_mac_metadata,
        ),
    };
    let create_options = CreateOptions {
        option,
        keep_options,
        pathname_editor: PathnameEditor::new(
            args.strip_components,
            PathTransformers::new(args.substitutions, args.transforms),
            args.absolute_paths,
        ),
    };

    // NOTE: "-" is not supported for update mode
    let mut file = args.file;
    file.take_if(|it| it == "-");
    let archive_path = match file.take() {
        Some(p) => current_dir.join(p),
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "update mode requires a file-based archive",
            )
            .into());
        }
    };
    if !archive_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not exists", archive_path.display()),
        )
        .into());
    }

    let mut files = args.files;
    if let Some(path) = args.files_from {
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

    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
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
        missing_ctime: MissingTimePolicy::Include,
        missing_mtime: MissingTimePolicy::Include,
    }
    .resolve()?;
    let collect_options = CollectOptions {
        recursive: !args.no_recursive,
        keep_dir: !args.no_keep_dir,
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

    let archives = collect_split_archives(&archive_path)?;

    let mut temp_file =
        NamedTempFile::new(|| archive_path.parent().unwrap_or_else(|| ".".as_ref()))?;
    let mut out_archive = Archive::write_header(temp_file.as_file_mut())?;

    let mut source = SplitArchiveReader::new(archives)?;
    run_update_archive(
        &mut source,
        password,
        &create_options,
        target_items,
        false,
        &mut out_archive,
        TransformStrategyUnSolid,
        args.verbose,
    )?;
    out_archive.finalize()?;
    drop(source);

    temp_file.persist(archive_path.remove_part())?;

    Ok(())
}
