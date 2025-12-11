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
            AclStrategy, CreateOptions, KeepOptions, OwnerOptions, PathFilter, PathTransformers,
            PathnameEditor, PermissionStrategy, TimeFilterResolver, TimeOptions, TimestampStrategy,
            XattrStrategy, collect_items, collect_split_archives, entry_option,
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
use pna::{
    Archive, DataKind, EntryBuilder, NormalEntry, ReadOptions, SolidEntryBuilder, WriteOptions,
};
use std::{
    env, fs,
    io::{self, Read, Write},
    path::PathBuf,
    sync::Arc,
    time::SystemTime,
};

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
    #[arg(short, long, help = "Create archive")]
    create: bool,
    #[arg(short = 'x', long, help = "Extract archive")]
    extract: bool,
    #[arg(short = 't', long, help = "List files in archive")]
    list: bool,
    #[arg(long, help = "Append files to archive")]
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
        help = "Archiving the permissions of the files (unstable on Windows)"
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
        help = "Archiving the extended attributes of the files"
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
        help = "Archiving the acl of the files (unstable)"
    )]
    keep_acl: bool,
    #[arg(
        long,
        visible_aliases = ["no-preserve-acls", "no-acls"],
        requires = "unstable",
        help = "Do not archive acl of files. This is the inverse option of --keep-acl (unstable)"
    )]
    no_keep_acl: bool,
    #[arg(long, help = "Solid mode archive")]
    pub(crate) solid: bool,
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
        requires = "unstable",
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)"
    )]
    include: Option<Vec<String>>,
    #[arg(
        long,
        requires = "unstable",
        help = "Exclude path glob (unstable)",
        value_hint = ValueHint::AnyPath
    )]
    exclude: Option<Vec<String>>,
    #[arg(
        short = 'X',
        long,
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
    #[arg(long, help = "Output directory of extracted files", value_hint = ValueHint::DirPath)]
    pub(crate) out_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Remove the specified number of leading path elements. Path names with fewer elements will be silently skipped"
    )]
    strip_components: Option<usize>,
    #[arg(
        long,
        requires = "unstable",
        conflicts_with_all = ["uname", "uid", "numeric_owner"],
        help = "Use the provided owner, if uid is not provided, name can be either a user name or numeric id. See the --uname option for details (unstable)."
    )]
    owner: Option<NameIdPair>,
    #[arg(
        long,
        help = "On create, archiving user to the entries from given name. On extract, restore user from given name"
    )]
    pub(crate) uname: Option<String>,
    #[arg(
        long,
        help = "On create, archiving group to the entries from given name. On extract, restore group from given name"
    )]
    pub(crate) gname: Option<String>,
    #[arg(
        long,
        help = "On create, this overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id. On extract, this overrides the user id in the archive; the user name in the archive will be ignored"
    )]
    pub(crate) uid: Option<u32>,
    #[arg(
        long,
        help = "On create, this overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id. On extract, this overrides the group id in the archive; the group name in the archive will be ignored"
    )]
    pub(crate) gid: Option<u32>,
    #[arg(
        long,
        requires = "unstable",
        conflicts_with_all = ["gname", "gid", "numeric_owner"],
        help = "Use the provided group, if gid is not provided, name can be either a group name or numeric id. See the --gname option for details (unstable)."
    )]
    group: Option<NameIdPair>,
    #[arg(
        long,
        help = "This is equivalent to --uname \"\" --gname \"\". On create, it causes user and group names to not be stored in the archive. On extract, it causes user and group names in the archive to be ignored in favor of the numeric user and group ids."
    )]
    pub(crate) numeric_owner: bool,
    #[arg(long, help = "Overrides the creation time")]
    ctime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the creation time of the entries to the specified time by --ctime"
    )]
    clamp_ctime: bool,
    #[arg(long, help = "Overrides the access time")]
    atime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the access time of the entries to the specified time by --atime"
    )]
    clamp_atime: bool,
    #[arg(long, help = "Overrides the modification time")]
    mtime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the modification time of the entries to the specified time by --mtime"
    )]
    clamp_mtime: bool,
    #[arg(
        long,
        requires = "unstable",
        help = "Only include files and directories older than the specified date (unstable). This compares ctime entries."
    )]
    older_ctime: Option<DateTime>,
    #[arg(
        long,
        requires = "unstable",
        help = "Only include files and directories older than the specified date (unstable). This compares mtime entries."
    )]
    older_mtime: Option<DateTime>,
    #[arg(
        long,
        requires = "unstable",
        help = "Only include files and directories newer than the specified date (unstable). This compares ctime entries."
    )]
    newer_ctime: Option<DateTime>,
    #[arg(
        long,
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
        help = "changes the directory before adding the following files",
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

/// Represents a file argument that can be either a filesystem path or an archive inclusion.
/// Archive inclusions start with '@' and contain entries from an existing archive.
#[derive(Clone, Debug)]
enum FileArg {
    /// A regular filesystem path (file or directory)
    Path(String),
    /// An archive inclusion (@archive-path). The string is the path after removing '@'.
    /// Special case: "-" means read archive from stdin.
    Archive(String),
}

/// Parses file arguments to separate regular paths from archive inclusions.
/// Arguments starting with '@' are treated as archive paths (bsdtar compatibility).
/// Archive paths are canonicalized to absolute paths so they remain valid after
/// working directory changes (via -C option).
fn parse_file_args(files: &[String]) -> io::Result<Vec<FileArg>> {
    files
        .iter()
        .map(|arg| {
            if let Some(archive_path) = arg.strip_prefix('@') {
                // Don't canonicalize stdin ("-")
                if archive_path == "-" {
                    Ok(FileArg::Archive(archive_path.to_string()))
                } else {
                    // Canonicalize to absolute path so it remains valid after -C directory change
                    let canonical = fs::canonicalize(archive_path)?;
                    Ok(FileArg::Archive(canonical.to_string_lossy().to_string()))
                }
            } else {
                Ok(FileArg::Path(arg.to_string()))
            }
        })
        .collect()
}

/// Checks if the file arguments contain any archive inclusions.
fn has_archive_inclusions(file_args: &[FileArg]) -> bool {
    file_args
        .iter()
        .any(|arg| matches!(arg, FileArg::Archive(_)))
}

/// Extracts only the filesystem paths from file arguments (excluding archive inclusions).
fn extract_filesystem_paths(file_args: &[FileArg]) -> Vec<String> {
    file_args
        .iter()
        .filter_map(|arg| match arg {
            FileArg::Path(p) => Some(p.clone()),
            FileArg::Archive(_) => None,
        })
        .collect()
}

/// Appends entries from a source archive file to the output archive (non-solid mode).
/// For @- (stdin), reads from stdin instead of a file.
/// Note: All entries from the source archive are included (matching bsdtar behavior).
fn append_archive_to_normal<W: Write>(
    archive_path: &str,
    output: &mut Archive<W>,
    verbose: bool,
) -> anyhow::Result<()> {
    // Handle @- for stdin
    let reader: Box<dyn Read> = if archive_path == "-" {
        Box::new(io::stdin().lock())
    } else {
        Box::new(fs::File::open(archive_path)?)
    };

    let mut source = Archive::read_header(io::BufReader::with_capacity(64 * 1024, reader))?;

    for entry in source.raw_entries() {
        let entry = entry?;

        if verbose {
            eprintln!("a (from @{})", archive_path);
        }

        output.add_entry(entry)?;
    }
    Ok(())
}

/// Appends entries from a source archive file to a solid archive builder.
/// Entries are decoded and re-encoded into the solid block.
fn append_archive_to_solid_builder(
    archive_path: &str,
    builder: &mut SolidEntryBuilder,
    password: Option<&[u8]>,
    verbose: bool,
) -> anyhow::Result<()> {
    // Handle @- for stdin
    let reader: Box<dyn Read> = if archive_path == "-" {
        Box::new(io::stdin().lock())
    } else {
        Box::new(fs::File::open(archive_path)?)
    };

    let mut source = Archive::read_header(io::BufReader::with_capacity(64 * 1024, reader))?;

    for entry in source.entries_with_password(password) {
        let entry = entry?;

        if verbose {
            eprintln!(
                "a {} (from @{})",
                entry.header().path().as_path().display(),
                archive_path
            );
        }

        // Re-create entry for solid mode
        let built = create_entry_from_normal_entry(&entry, password)?;
        builder.add_entry(built)?;
    }
    Ok(())
}

/// Creates a new NormalEntry from an existing NormalEntry by decoding and re-encoding.
/// This is needed for solid mode where we need to decode encrypted/compressed entries
/// and re-encode them with WriteOptions::store().
fn create_entry_from_normal_entry<T: AsRef<[u8]>>(
    entry: &NormalEntry<T>,
    password: Option<&[u8]>,
) -> io::Result<NormalEntry> {
    let header = entry.header();
    let metadata = entry.metadata();
    let xattrs = entry.xattrs();

    let mut builder = match header.data_kind() {
        DataKind::File => {
            let mut b = EntryBuilder::new_file(header.path().clone(), WriteOptions::store())?;
            let mut reader = entry.reader(ReadOptions::with_password(password))?;
            io::copy(&mut reader, &mut b)?;
            b
        }
        DataKind::Directory => EntryBuilder::new_dir(header.path().clone()),
        DataKind::SymbolicLink => {
            let mut reader = entry.reader(ReadOptions::with_password(password))?;
            let mut target = String::new();
            reader.read_to_string(&mut target)?;
            EntryBuilder::new_symlink(header.path().clone(), target.into())?
        }
        DataKind::HardLink => {
            let mut reader = entry.reader(ReadOptions::with_password(password))?;
            let mut target = String::new();
            reader.read_to_string(&mut target)?;
            EntryBuilder::new_hard_link(header.path().clone(), target.into())?
        }
    };

    // Apply metadata
    if let Some(created) = metadata.created() {
        builder.created(created);
    }
    if let Some(modified) = metadata.modified() {
        builder.modified(modified);
    }
    if let Some(accessed) = metadata.accessed() {
        builder.accessed(accessed);
    }
    if let Some(permission) = metadata.permission() {
        builder.permission(permission.clone());
    }

    // Apply xattrs
    for xattr in xattrs {
        builder.add_xattr(xattr.clone());
    }

    builder.build()
}

/// Creates an archive with support for @archive inclusions (bsdtar compatibility).
/// Processes file arguments in order, interleaving filesystem entries and archive inclusions.
fn create_archive_with_inclusions<W, F>(
    mut get_writer: F,
    CreationContext {
        write_option,
        keep_options,
        owner_options,
        time_options,
        solid,
        pathname_editor,
    }: CreationContext,
    target_items: Vec<(PathBuf, crate::command::core::StoreAs)>,
    file_args: &[FileArg],
    password: Option<&[u8]>,
    verbose: bool,
) -> anyhow::Result<()>
where
    W: Write,
    F: FnMut() -> io::Result<W>,
{
    // Build entries from filesystem using rayon (like create_archive_file does)
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
        time_options,
        pathname_editor,
    };

    rayon::scope_fifo(|s| {
        for file in target_items {
            let tx = tx.clone();
            let create_options = create_options.clone();
            s.spawn_fifo(move |_| {
                log::debug!("Adding: {}", file.0.display());
                tx.send(crate::command::core::create_entry(&file, &create_options))
                    .unwrap_or_else(|e| log::error!("{e}: {}", file.0.display()));
            })
        }
        drop(tx);
    });

    // Collect all filesystem entries
    let filesystem_entries: Vec<_> = rx.into_iter().collect();
    let mut entry_iter = filesystem_entries.into_iter();

    let file = get_writer()?;

    if solid {
        // Solid mode: re-encode all entries into the solid block
        let mut builder = SolidEntryBuilder::new(write_option)?;

        for arg in file_args {
            match arg {
                FileArg::Path(_) => {
                    // Write corresponding filesystem entry
                    if let Some(entry_result) = entry_iter.next() {
                        if let Some(entry) = entry_result? {
                            if verbose {
                                eprintln!("a {}", entry.header().path().as_path().display());
                            }
                            builder.add_entry(entry)?;
                        }
                    }
                }
                FileArg::Archive(archive_path) => {
                    // Include entries from source archive
                    append_archive_to_solid_builder(archive_path, &mut builder, password, verbose)?;
                }
            }
        }

        // Finalize solid archive
        let solid_entry = builder.build()?;
        let mut archive = Archive::write_header(file)?;
        archive.add_entry(solid_entry)?;
        archive.finalize()?;
    } else {
        // Normal mode: copy entries directly
        let mut writer = Archive::write_header(file)?;

        for arg in file_args {
            match arg {
                FileArg::Path(_) => {
                    // Write corresponding filesystem entry
                    if let Some(entry_result) = entry_iter.next() {
                        if let Some(entry) = entry_result? {
                            if verbose {
                                eprintln!("a {}", entry.header().path().as_path().display());
                            }
                            writer.add_entry(entry)?;
                        }
                    }
                }
                FileArg::Archive(archive_path) => {
                    // Include entries from source archive
                    append_archive_to_normal(archive_path, &mut writer, verbose)?;
                }
            }
        }

        writer.finalize()?;
    }

    Ok(())
}

fn run_stdio(args: StdioCommand) -> anyhow::Result<()> {
    if let Some(format) = &args.format {
        log::debug!(
            "Warning: Option '--format {format}' is accepted for compatibility but will be ignored."
        );
    }
    if args.read_full_blocks {
        log::debug!(
            "Warning: Option '--read-full-blocks' is accepted for compatibility but will be ignored."
        );
    }
    if args.ignore_zeros {
        log::debug!(
            "Warning: Option '--ignore-zeros' is accepted for compatibility but will be ignored."
        );
    }
    if args.safe_writes {
        log::debug!(
            "Warning: Option '--safe-writes' is accepted for compatibility but will be ignored."
        );
    }
    if args.no_safe_writes {
        log::debug!(
            "Warning: Option '--no-safe-writes' is accepted for compatibility but will be ignored."
        );
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
    let password = ask_password(args.password.clone())?;
    check_password(&password, &args.cipher);
    // NOTE: "-" will use stdout
    let mut file = args.file.clone();
    file.take_if(|it| it == "-");
    let archive_file = file.take().map(|p| current_dir.join(p));
    let mut files = args.files.clone();
    if let Some(ref path) = args.files_from {
        files.extend(read_paths(path, args.null)?);
    }

    // Parse file arguments to detect @archive inclusions
    // This must happen before changing working directory so @archive paths are canonicalized
    let file_args = parse_file_args(&files)?;

    let mut exclude = args.exclude.clone().unwrap_or_default();
    if let Some(ref p) = args.exclude_from {
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

    // Extract only filesystem paths (exclude @archive inclusions) for collect_items
    let filesystem_paths = extract_filesystem_paths(&file_args);

    let target_items = collect_items(
        &filesystem_paths,
        !args.no_recursive,
        args.keep_dir,
        args.gitignore,
        args.nodump,
        args.follow_links,
        args.follow_command_links,
        args.one_file_system,
        &filter,
        &time_filters,
    )?;

    let password = password.as_deref();
    let cli_option = entry_option(
        args.compression.clone(),
        args.cipher.clone(),
        args.hash.clone(),
        password,
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

    // Check if there are any @archive inclusions
    if has_archive_inclusions(&file_args) {
        // Check for @- stdin conflict with -f - (stdout output)
        let uses_stdout = archive_file.is_none();
        let uses_stdin_archive = file_args
            .iter()
            .any(|arg| matches!(arg, FileArg::Archive(p) if p == "-"));
        if uses_stdout && uses_stdin_archive {
            anyhow::bail!("Cannot use @- (read archive from stdin) when writing to stdout (-f -)");
        }

        // Use custom archive creation with @archive support
        if let Some(file) = archive_file {
            create_archive_with_inclusions(
                || utils::fs::file_create(&file, args.overwrite),
                creation_context,
                target_items,
                &file_args,
                password,
                args.verbose,
            )
        } else {
            create_archive_with_inclusions(
                || Ok(io::stdout().lock()),
                creation_context,
                target_items,
                &file_args,
                password,
                args.verbose,
            )
        }
    } else {
        // No @archive inclusions, use the existing optimized path
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
    let mut file = args.file.clone();
    file.take_if(|it| it == "-");
    let archive_path = file.take().map(|p| current_dir.join(p));
    let mut files = args.files.clone();
    if let Some(ref path) = args.files_from {
        files.extend(read_paths(path, args.null)?);
    }

    // Parse file arguments to detect @archive inclusions
    // This must happen before changing working directory so @archive paths are canonicalized
    let file_args = parse_file_args(&files)?;

    let mut exclude = args.exclude.clone().unwrap_or_default();
    if let Some(ref p) = args.exclude_from {
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
    // Extract only filesystem paths (exclude @archive inclusions)
    let filesystem_paths = extract_filesystem_paths(&file_args);

    if let Some(file) = &archive_path {
        let mut archive = open_archive_then_seek_to_end(file)?;
        let target_items = collect_items(
            &filesystem_paths,
            args.recursive,
            args.keep_dir,
            args.gitignore,
            args.nodump,
            args.follow_links,
            args.follow_command_links,
            args.one_file_system,
            &filter,
            &time_filters,
        )?;

        // Check if there are any @archive inclusions
        if has_archive_inclusions(&file_args) {
            // Append entries from @archive sources first
            for arg in &file_args {
                if let FileArg::Archive(archive_path) = arg {
                    append_archive_to_normal(archive_path, &mut archive, args.verbose)?;
                }
            }
        }

        run_append_archive(&create_options, archive, target_items)
    } else {
        // Check for @- stdin conflict
        let uses_stdin_archive = file_args
            .iter()
            .any(|arg| matches!(arg, FileArg::Archive(p) if p == "-"));
        if uses_stdin_archive {
            anyhow::bail!(
                "Cannot use @- (read archive from stdin) when reading input archive from stdin (-f -)"
            );
        }

        let target_items = collect_items(
            &filesystem_paths,
            args.recursive,
            args.keep_dir,
            args.gitignore,
            args.nodump,
            args.follow_links,
            args.follow_command_links,
            args.one_file_system,
            &filter,
            &time_filters,
        )?;
        let mut output_archive = Archive::write_header(io::stdout().lock())?;
        {
            let mut input_archive = Archive::read_header(io::stdin().lock())?;
            for entry in input_archive.raw_entries() {
                output_archive.add_entry(entry?)?;
            }
        }

        // Check if there are any @archive inclusions
        if has_archive_inclusions(&file_args) {
            // Append entries from @archive sources
            for arg in &file_args {
                if let FileArg::Archive(archive_path) = arg {
                    append_archive_to_normal(archive_path, &mut output_archive, args.verbose)?;
                }
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
