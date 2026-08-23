use crate::{
    cli::{
        ArchiveFileArgs, ArchiveOutputArgs, CipherAlgorithmArgs, CompressionAlgorithmArgs,
        DateTime, FileOperands, HashAlgorithmArgs, MissingTimePolicy, PasswordArgs,
    },
    command::{
        Command, ask_password,
        core::{
            AclStrategy, ArchiveSource, CollectOptions, CollectedItem, CreateOptions,
            FflagsStrategy, KeepOptions, MacMetadataStrategy, PathFilter, PathTransformers,
            PathnameEditor, PermissionStrategyResolver, SourceConsumer, TimeFilterResolver,
            TimeFilters, TimestampStrategyResolver, XattrStrategy,
            archive_destination::{ArchiveDestination, SinkConsumer, resolve_append_destination},
            collect_items_from_paths, drain_entry_results, entry_option,
            re::{bsd::SubstitutionRule, gnu::TransformRule},
            read_paths, read_paths_stdin, run_across_archive_readers, spawn_entry_results,
        },
    },
    utils::{PathPartExt, VCS_FILES, fs::HardlinkResolver},
};
use clap::{ArgAction, ArgGroup, Parser, ValueHint, builder::ArgPredicate};
use pna::{Archive, prelude::*};
use std::{
    fs, io,
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
};

#[derive(Parser, Clone, Debug)]
#[command(
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
    group(ArgGroup::new("keep-timestamp-flag").args(["keep_timestamp", "no_keep_timestamp"])),
    group(ArgGroup::new("ctime-older-than-source").args(["older_ctime", "older_ctime_than"])),
    group(ArgGroup::new("ctime-newer-than-source").args(["newer_ctime", "newer_ctime_than"])),
    group(ArgGroup::new("mtime-older-than-source").args(["older_mtime", "older_mtime_than"])),
    group(ArgGroup::new("mtime-newer-than-source").args(["newer_mtime", "newer_mtime_than"])),
)]
pub(crate) struct AppendCommand {
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
        default_value_t = true,
        default_value_if("no_recursive", ArgPredicate::Equals("true".into()), "false"),
        conflicts_with = "no_recursive"
    )]
    recursive: bool,
    #[arg(
        long,
        visible_alias = "no-recursion",
        action = ArgAction::SetTrue,
        help = "Do not recursively add directories to the archives. This is the inverse option of --recursive"
    )]
    no_recursive: (),
    #[arg(
        long,
        help = "Include directories in archive (default)",
        default_value_t = true,
        default_value_if("no_keep_dir", ArgPredicate::Equals("true".into()), "false"),
        conflicts_with = "no_keep_dir"
    )]
    keep_dir: bool,
    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Do not archive directories. This is the inverse option of --keep-dir"
    )]
    no_keep_dir: (),
    #[arg(
        long = "preserve-timestamps",
        visible_alias = "keep-timestamp",
        help = "Preserve file timestamps"
    )]
    keep_timestamp: bool,
    #[arg(
        long = "no-preserve-timestamps",
        visible_alias = "no-keep-timestamp",
        help = "Do not archive timestamp of files. This is the inverse option of --preserve-timestamps"
    )]
    pub(crate) no_keep_timestamp: bool,
    #[arg(
        long = "preserve-permissions",
        visible_alias = "keep-permission",
        conflicts_with = "no_keep_permission",
        help = "Preserve file permissions"
    )]
    #[cfg_attr(windows, arg(requires = "unstable", help_heading = "Unstable Options"))]
    keep_permission: bool,
    #[arg(
        long = "no-preserve-permissions",
        visible_alias = "no-keep-permission",
        action = ArgAction::SetTrue,
        help = "Do not archive permissions of files. This is the inverse option of --preserve-permissions"
    )]
    #[cfg_attr(windows, arg(requires = "unstable", help_heading = "Unstable Options"))]
    no_keep_permission: (),
    #[arg(
        long = "preserve-xattrs",
        visible_alias = "keep-xattr",
        conflicts_with = "no_keep_xattr",
        help = "Preserve extended attributes"
    )]
    keep_xattr: bool,
    #[arg(
        long = "no-preserve-xattrs",
        visible_alias = "no-keep-xattr",
        action = ArgAction::SetTrue,
        help = "Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs"
    )]
    pub(crate) no_keep_xattr: (),
    #[arg(
        long = "preserve-acls",
        visible_alias = "keep-acl",
        requires = "unstable",
        help_heading = "Unstable Options",
        conflicts_with = "no_keep_acl",
        help = "Preserve ACLs"
    )]
    keep_acl: bool,
    #[arg(
        long = "no-preserve-acls",
        visible_alias = "no-keep-acl",
        requires = "unstable",
        help_heading = "Unstable Options",
        action = ArgAction::SetTrue,
        help = "Do not archive ACLs. This is the inverse option of --preserve-acls"
    )]
    no_keep_acl: (),
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
        requires_all = ["unstable", "file"],
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
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
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
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) hash: HashAlgorithmArgs,
    #[command(flatten)]
    pub(crate) archive: ArchiveFileArgs,
    #[command(flatten)]
    output: ArchiveOutputArgs,
    #[command(flatten)]
    pub(crate) files: FileOperands,
}

impl Command for AppendCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        append_to_archive(self, ctx.umask())
    }
}

#[hooq::hooq(anyhow)]
fn append_to_archive(
    args: AppendCommand,
    umask: crate::command::core::Umask,
) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let source_arg = args.archive.source();
    if let ArchiveSource::File(path) = &source_arg
        && !path.exists()
    {
        anyhow::bail!("{} is not exists", path.display());
    }
    let password = password.as_deref();
    let option = entry_option(args.compression, args.cipher, args.hash, password);
    let (mode_strategy, owner_strategy) = PermissionStrategyResolver {
        keep_permission: args.keep_permission,
        same_owner: true, // Creation always stores ownership; same_owner only matters for extraction.
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
        xattr_strategy: XattrStrategy::from_flag(args.keep_xattr),
        acl_strategy: AclStrategy::from_flag(args.keep_acl),
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
            false,
        ),
    };

    let mut files = args.files.files;
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
    let mut resolver = HardlinkResolver::new(collect_options.follow_links);
    let target_items = collect_items_from_paths(&files, &collect_options, &mut resolver)?
        .into_iter()
        .map(CollectedItem::Filesystem)
        .collect::<Vec<_>>();

    let destination =
        resolve_append_destination(&source_arg, args.output.output, args.output.overwrite)?;
    match destination {
        ArchiveDestination::InPlace(path) => {
            let archive = open_archive_then_seek_to_end(path, false)?;
            run_append_archive(
                &create_options,
                archive,
                target_items,
                &filter,
                &time_filters,
                password,
                false,
                false,
            )
        }
        destination => destination.open_with(
            umask,
            AppendRewrite {
                source: source_arg,
                create_options: &create_options,
                target_items,
                filter: &filter,
                time_filters: &time_filters,
                password,
            },
        ),
    }
}

/// Copies the base archive from `source` and appends the collected filesystem items into
/// the opened destination.
struct AppendRewrite<'a> {
    source: ArchiveSource,
    create_options: &'a CreateOptions,
    target_items: Vec<CollectedItem>,
    filter: &'a PathFilter<'a>,
    time_filters: &'a TimeFilters,
    password: Option<&'a [u8]>,
}

impl SinkConsumer for AppendRewrite<'_> {
    type Output = ();

    fn consume<W: io::Write>(self, mut writer: W) -> anyhow::Result<()> {
        self.source.open()?.consume(AppendBase {
            writer: &mut writer,
            create_options: self.create_options,
            target_items: self.target_items,
            filter: self.filter,
            time_filters: self.time_filters,
            password: self.password,
        })
    }
}

/// Copies the base archive's raw entries into the writer, then appends the collected items.
struct AppendBase<'a, W> {
    writer: &'a mut W,
    create_options: &'a CreateOptions,
    target_items: Vec<CollectedItem>,
    filter: &'a PathFilter<'a>,
    time_filters: &'a TimeFilters,
    password: Option<&'a [u8]>,
}

impl<W: io::Write> SourceConsumer for AppendBase<'_, W> {
    type Output = anyhow::Result<()>;

    fn readers<R: io::Read, I: Iterator<Item = R>>(self, readers: I) -> anyhow::Result<()> {
        run_rewrite_append_archive(
            readers,
            self.writer,
            self.create_options,
            self.target_items,
            self.filter,
            self.time_filters,
            self.password,
            false,
            false,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_rewrite_append_archive<R: io::Read, W: io::Write>(
    readers: impl IntoIterator<Item = R>,
    writer: W,
    create_options: &CreateOptions,
    target_items: Vec<CollectedItem>,
    filter: &PathFilter<'_>,
    time_filters: &TimeFilters,
    password: Option<&[u8]>,
    verbose: bool,
    allow_concatenated_archives: bool,
) -> anyhow::Result<()> {
    let mut output_archive = Archive::write_header(writer)?;
    run_across_archive_readers(
        readers,
        |input_archive| {
            for entry in input_archive.raw_entries() {
                output_archive.add_entry(entry?)?;
            }
            Ok(())
        },
        allow_concatenated_archives,
    )?;
    run_append_archive(
        create_options,
        output_archive,
        target_items,
        filter,
        time_filters,
        password,
        verbose,
        allow_concatenated_archives,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_append_archive(
    create_options: &CreateOptions,
    mut archive: Archive<impl io::Write>,
    target_items: Vec<CollectedItem>,
    filter: &PathFilter<'_>,
    time_filters: &TimeFilters,
    password: Option<&[u8]>,
    verbose: bool,
    allow_concatenated_archives: bool,
) -> anyhow::Result<()> {
    let rx = spawn_entry_results(
        target_items,
        create_options,
        filter,
        time_filters,
        password,
        allow_concatenated_archives,
    );
    drain_entry_results(rx, |entry| {
        if verbose {
            eprintln!("a {}", entry.name());
        }
        archive.add_entry(entry)
    })?;
    archive.finalize()?;
    Ok(())
}

#[inline]
pub(crate) fn open_archive_then_seek_to_end(
    path: impl AsRef<Path>,
    allow_concatenated_archives: bool,
) -> io::Result<Archive<fs::File>> {
    if !allow_concatenated_archives {
        return Archive::open_multipart_for_append(path, |base, index| base.with_part(index));
    }

    let base = path.as_ref();
    let mut current_path = base.to_path_buf();
    let mut current_offset = 0;
    let mut part_index = 1;

    loop {
        let mut scan_file = fs::File::open(&current_path)?;
        if current_offset != 0 {
            scan_file.seek(SeekFrom::Start(current_offset))?;
        }
        let mut archive = Archive::read_header(scan_file)?;
        for entry in archive.raw_entries() {
            entry?;
        }
        if archive.has_next_archive() {
            part_index += 1;
            current_path = base.with_part(part_index);
            current_offset = 0;
            continue;
        }

        let mut scan_file = archive.into_inner();
        let next_offset = scan_file.stream_position()?;
        match Archive::read_header(scan_file) {
            Ok(_) => {
                current_offset = next_offset;
            }
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                let mut file = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&current_path)?;
                if current_offset != 0 {
                    file.seek(SeekFrom::Start(current_offset))?;
                }
                let mut archive = Archive::read_header(file)?;
                archive.seek_to_end()?;
                return Ok(archive);
            }
            Err(err) => return Err(err),
        }
    }
}
