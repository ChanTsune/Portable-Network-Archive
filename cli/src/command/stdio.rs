use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, DateTime, FileArgs, HashAlgorithmArgs,
        PasswordArgs, SolidEntriesTransformStrategyArgs,
    },
    command::{
        append::{open_archive_then_seek_to_end, run_append_archive},
        ask_password, check_password,
        commons::{
            collect_items, collect_split_archives, ensure_hardlinks_complete, entry_option,
            read_paths, CreateOptions, Exclude, KeepOptions, OwnerOptions, PathTransformers,
            TimeOptions,
        },
        concat::{append_archives_into_existing, run_concat_from_stdio, ConcatFromStdioArgs},
        create::{create_archive_file, CreationContext},
        delete::{run_delete_from_stdio, DeleteFromStdioArgs},
        extract::{run_extract_archive_reader, OutputOption},
        list::{ListOptions, TimeField, TimeFormat},
        update::{run_update_from_stdio, UpdateFromStdioArgs},
        Command,
    },
    utils::{
        self,
        re::{bsd::SubstitutionRule, gnu::TransformRule},
        GlobPatterns, VCS_FILES,
    },
};
use anyhow::bail;
use clap::{ArgGroup, Args, ValueHint};
use pna::Archive;
use std::{
    env, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Args, Clone, Debug)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["keep_acl"]).requires("unstable")),
    group(ArgGroup::new("keep-old-files-group").args(["keep_old_files"])),
    group(ArgGroup::new("keep-newer-files-group").args(["keep_newer_files"])),
    group(ArgGroup::new("bundled-flags").args(["create", "extract", "list"]).required(true)),
    group(ArgGroup::new("include-group").args(["include"])),
    group(ArgGroup::new("exclude-group").args(["exclude"])),
    group(ArgGroup::new("exclude-from-group").args(["exclude_from"])),
    group(ArgGroup::new("files-from-group").args(["files_from"])),
    group(
        ArgGroup::new("from-input")
            .args(["files_from", "exclude_from"])
            .multiple(true)
    ),
    group(ArgGroup::new("null-requires").arg("null").requires("from-input")),
    group(ArgGroup::new("gitignore-group").args(["gitignore"])),
    group(ArgGroup::new("substitution-group").args(["substitutions"])),
    group(ArgGroup::new("transform-group").args(["transforms"])),
    group(ArgGroup::new("path-transform").args(["substitutions", "transforms"])),
    group(ArgGroup::new("owner-flag").args(["same_owner", "no_same_owner"])),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
    group(ArgGroup::new("recursive-flag").args(["recursive", "no_recursive"])),
    group(ArgGroup::new("keep-dir-flag").args(["keep_dir", "no_keep_dir"])),
    group(ArgGroup::new("action-flags").args(["create", "extract", "list", "append", "update", "delete"])),
    group(ArgGroup::new("ctime-flag").args(["clamp_ctime"]).requires("ctime")),
    group(ArgGroup::new("mtime-flag").args(["clamp_mtime"]).requires("mtime")),
    group(ArgGroup::new("atime-flag").args(["clamp_atime"]).requires("atime")),
    group(ArgGroup::new("exclude-vcs-group").args(["exclude_vcs"])),
    group(ArgGroup::new("unstable-follow_command_links").args(["follow_command_links"]).requires("unstable")),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission"]).requires("unstable")),
))]
pub(crate) struct StdioCommand {
    #[arg(short, long, help = "Create archive")]
    create: bool,
    #[arg(short = 'x', long, visible_alias = "get", help = "Extract archive")]
    extract: bool,
    #[arg(short = 't', long, help = "List files in archive")]
    list: bool,
    #[arg(
        short = 'r',
        long,
        help = "Append files to archive (bsdtar -r equivalent; compression flags are unsupported)"
    )]
    append: bool,
    #[arg(
        short = 'A',
        long = "append-to",
        help = "Concatenate archives into the target archive (bsdtar -A equivalent)"
    )]
    append_to: bool,
    #[arg(
        short = 'u',
        long,
        help = "Update existing archive entries if sources are newer (bsdtar -u equivalent)"
    )]
    update: bool,
    #[arg(
        short = 'd',
        long,
        help = "Delete entries from an archive (bsdtar -d equivalent)"
    )]
    delete: bool,
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
    #[arg(long, help = "Archiving the directories")]
    keep_dir: bool,
    #[arg(
        long,
        help = "Do not archive directories. This is the inverse option of --keep-dir"
    )]
    no_keep_dir: bool,
    #[arg(
        long = "one-file-system",
        help = "When recursing, stay on the same file system as the source path"
    )]
    one_file_system: bool,
    #[arg(long, help = "Do not overwrite existing files when extracting")]
    keep_old_files: bool,
    #[arg(
        long,
        help = "Only overwrite if archive entry is newer than existing file"
    )]
    keep_newer_files: bool,
    #[arg(
        short = 'l',
        long = "check-links",
        help = "Fail if any hard link targets referenced on disk are missing from the archive input set"
    )]
    check_links: bool,
    #[arg(
        long,
        visible_alias = "preserve-timestamps",
        help = "Archiving the timestamp of the files"
    )]
    keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Archiving the permissions of the files (unstable on Windows)"
    )]
    keep_permission: bool,
    #[arg(
        long,
        visible_alias = "preserve-xattrs",
        help = "Archiving the extended attributes of the files"
    )]
    keep_xattr: bool,
    #[arg(
        long,
        visible_alias = "preserve-acls",
        help = "Archiving the acl of the files (unstable)"
    )]
    keep_acl: bool,
    #[arg(long, help = "Solid mode archive")]
    pub(crate) solid: bool,
    #[arg(
        short = 'B',
        long = "read-full-blocks",
        help = "Compatibility flag; accepted but ignored"
    )]
    read_full_blocks: bool,
    #[arg(
        short = 'b',
        long = "block-size",
        value_hint = ValueHint::Other,
        help = "Compatibility option; accepted but ignored",
        num_args = 1
    )]
    block_size: Option<u16>,
    #[arg(long, help = "Run operations inside a chroot (currently unsupported)")]
    chroot: Option<PathBuf>,
    #[arg(long, help = "Compatibility option; accepted but ignored")]
    clear_nochange_fflags: bool,
    #[arg(long, help = "Compatibility option; accepted but ignored")]
    fflags: bool,
    #[arg(long, help = "Requested archive format (compatibility only)")]
    format: Option<String>,
    #[arg(
        long = "options",
        value_delimiter = ',',
        help = "Format-specific options (compatibility only)"
    )]
    format_options: Vec<String>,
    #[arg(
        long = "use-compress-program",
        value_hint = ValueHint::CommandString,
        help = "Use external compressor (compatibility only; ignored)"
    )]
    use_compress_program: Option<String>,
    #[arg(
        short = 'a',
        long = "auto-compress",
        help = "Choose compression based on archive filename (bsdtar -a equivalent)"
    )]
    auto_compress: bool,
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
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions"
    )]
    include: Option<Vec<String>>,
    #[arg(
        long,
        help = "Exclude path glob",
        value_hint = ValueHint::AnyPath
    )]
    pub(crate) exclude: Option<Vec<String>>,
    #[arg(
        short = 'X',
        long,
        help = "Read exclude patterns from given path",
        value_hint = ValueHint::FilePath
    )]
    exclude_from: Option<String>,
    #[arg(long, help = "Exclude vcs files")]
    exclude_vcs: bool,
    #[arg(long, help = "Ignore files from .gitignore")]
    pub(crate) gitignore: bool,
    #[arg(long, visible_aliases = ["dereference"], help = "Follow symbolic links")]
    follow_links: bool,
    #[arg(
        short = 'H',
        long,
        help = "Follow symbolic links named on the command line"
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
        short = 'T',
        long,
        help = "Read archive member names from file",
        value_hint = ValueHint::FilePath
    )]
    pub(crate) files_from: Option<String>,
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
        help = "changes the directory before adding the following files",
        value_hint = ValueHint::DirPath
    )]
    working_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Allow extracting symbolic links and hard links that contain root or parent paths"
    )]
    allow_unsafe_links: bool,
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
}

impl Command for StdioCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        run_stdio(self)
    }
}

fn run_stdio(args: StdioCommand) -> anyhow::Result<()> {
    if args.read_full_blocks {
        log::warn!("--read-full-blocks/-B is accepted for compatibility but has no effect");
    }
    if args.block_size.is_some() {
        log::warn!("--block-size/-b is accepted for compatibility but has no effect");
    }
    if let Some(path) = &args.chroot {
        log::warn!("--chroot is currently unsupported and will return an error");
        bail!(
            "--chroot is not supported yet: requested root {}",
            path.display()
        );
    }
    if args.clear_nochange_fflags {
        log::warn!("--clear-nochange-fflags is accepted for compatibility but has no effect");
    }
    if args.fflags {
        log::warn!("--fflags is accepted for compatibility but has no effect");
    }
    if let Some(fmt) = &args.format {
        log::warn!(
            "--format={} is accepted for compatibility but PNA format is always used",
            fmt
        );
    }
    if !args.format_options.is_empty() {
        log::warn!(
            "--options is accepted for compatibility but ignored: {:?}",
            args.format_options
        );
    }
    if let Some(program) = &args.use_compress_program {
        log::warn!(
            "--use-compress-program={} is accepted for compatibility but ignored",
            program
        );
    }

    if args.create {
        run_create_archive(args)
    } else if args.extract {
        run_extract_archive(args)
    } else if args.list {
        run_list_archive(args)
    } else if args.append_to {
        run_append_to(args)
    } else if args.delete {
        run_delete(args)
    } else if args.update {
        if args.compression.explicitly_set() || args.auto_compress {
            bail!(
                "compression flags cannot be combined with update/-u; the archive's existing compression is preserved"
            );
        }
        run_update(args)
    } else if args.append {
        if args.compression.explicitly_set() || args.auto_compress {
            bail!(
                "compression flags cannot be combined with append/-r; the archive's existing compression is preserved"
            );
        }
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
    let mut archive_sources_raw = Vec::new();
    files.retain(|entry| {
        if let Some(rest) = entry.strip_prefix('@') {
            archive_sources_raw.push(rest.to_string());
            false
        } else {
            true
        }
    });
    let exclude = {
        let mut exclude = args.exclude.unwrap_or_default();
        if let Some(p) = args.exclude_from {
            exclude.extend(read_paths(p, args.null)?);
        }
        if args.exclude_vcs {
            exclude.extend(VCS_FILES.iter().map(|it| String::from(*it)))
        }
        Exclude {
            include: args.include.unwrap_or_default().into(),
            exclude: exclude.into(),
        }
    };
    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
    }
    let base_dir = env::current_dir()?;
    let archive_sources = archive_sources_raw
        .into_iter()
        .map(PathBuf::from)
        .map(|src| {
            if src.is_absolute() {
                src
            } else {
                base_dir.join(src)
            }
        })
        .collect::<Vec<_>>();
    let target_items = collect_items(
        &files,
        !args.no_recursive,
        args.keep_dir,
        args.gitignore,
        args.follow_links,
        args.follow_command_links,
        args.one_file_system,
        &exclude,
    )?;

    if args.check_links {
        ensure_hardlinks_complete(&target_items, args.follow_links)?;
    }

    let mut compression = args.compression.clone();
    if args.auto_compress {
        if let Some(ref path) = archive_file {
            apply_auto_compress(&mut compression, path);
        } else {
            log::warn!("--auto-compress ignored when writing to stdout");
        }
    }

    let password = password.as_deref();
    let cli_option = entry_option(compression, args.cipher, args.hash, password);
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
    let path_transformers = PathTransformers::new(args.substitutions, args.transforms);
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
        path_transformers,
    };
    match archive_file {
        Some(file) => {
            create_archive_file(
                || utils::fs::file_create(&file, args.overwrite),
                creation_context,
                target_items,
            )?;
            if !archive_sources.is_empty() {
                append_archives_into_existing(&file, &archive_sources)?;
            }
            Ok(())
        }
        None => {
            if !archive_sources.is_empty() {
                bail!("@archive inputs are not supported when writing to stdout");
            }
            create_archive_file(|| Ok(io::stdout().lock()), creation_context, target_items)
        }
    }
}

fn run_extract_archive(args: StdioCommand) -> anyhow::Result<()> {
    let current_dir = env::current_dir()?;
    let password = ask_password(args.password)?;

    let exclude = {
        let mut exclude = args.exclude.unwrap_or_default();
        if let Some(p) = args.exclude_from {
            exclude.extend(read_paths(p, args.null)?);
        }
        if args.exclude_vcs {
            exclude.extend(VCS_FILES.iter().map(|it| String::from(*it)))
        }
        Exclude {
            include: args.include.unwrap_or_default().into(),
            exclude: exclude.into(),
        }
    };

    let out_option = OutputOption {
        overwrite: args.overwrite && !args.keep_old_files,
        keep_old_files: args.keep_old_files,
        keep_newer_files: args.keep_newer_files,
        allow_unsafe_links: args.allow_unsafe_links,
        strip_components: args.strip_components,
        out_dir: args.out_dir,
        exclude,
        keep_options: KeepOptions {
            keep_timestamp: args.keep_timestamp,
            keep_permission: args.keep_permission,
            keep_xattr: args.keep_xattr,
            keep_acl: args.keep_acl,
        },
        owner_options: OwnerOptions::new(
            args.uname,
            args.gname,
            args.uid,
            args.gid,
            args.numeric_owner,
        ),
        same_owner: !args.no_same_owner,
        path_transformers: PathTransformers::new(args.substitutions, args.transforms),
    };
    // NOTE: "-" will use stdin
    let mut file = args.file;
    file.take_if(|it| it == "-");
    let archive_path = file.take().map(|p| current_dir.join(p));
    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
    }
    if let Some(path) = archive_path {
        let archives = collect_split_archives(&path)?;
        run_extract_archive_reader(
            archives
                .into_iter()
                .map(|it| io::BufReader::with_capacity(64 * 1024, it)),
            args.files,
            || password.as_deref(),
            out_option,
        )
    } else {
        run_extract_archive_reader(
            std::iter::repeat_with(|| io::stdin().lock()),
            args.files,
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
        classify: false,
        format: None,
    };
    let files_globs = GlobPatterns::new(args.files.iter().map(|it| it.as_str()))?;

    let exclude = {
        let mut exclude = args.exclude.unwrap_or_default();
        if let Some(p) = args.exclude_from {
            exclude.extend(read_paths(p, args.null)?);
        }
        if args.exclude_vcs {
            exclude.extend(VCS_FILES.iter().map(|it| String::from(*it)))
        }
        Exclude {
            include: args.include.unwrap_or_default().into(),
            exclude: exclude.into(),
        }
    };
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
            exclude,
            list_options,
        )
    } else {
        crate::command::list::run_list_archive(
            std::iter::repeat_with(|| io::stdin().lock()),
            password.as_deref(),
            files_globs,
            exclude,
            list_options,
        )
    }
}

fn run_append_to(args: StdioCommand) -> anyhow::Result<()> {
    let StdioCommand {
        create: _,
        extract: _,
        list: _,
        append: _,
        append_to: _,
        update: _,
        delete: _,
        recursive: _,
        no_recursive: _,
        overwrite,
        keep_dir: _,
        no_keep_dir: _,
        keep_timestamp: _,
        keep_permission: _,
        keep_xattr: _,
        keep_acl: _,
        solid: _,
        compression: _,
        cipher: _,
        hash: _,
        password: _,
        include: _,
        exclude: _,
        exclude_from: _,
        exclude_vcs: _,
        gitignore: _,
        follow_links: _,
        follow_command_links: _,
        out_dir: _,
        strip_components: _,
        uname: _,
        gname: _,
        uid: _,
        gid: _,
        numeric_owner: _,
        ctime: _,
        clamp_ctime: _,
        atime: _,
        clamp_atime: _,
        mtime: _,
        clamp_mtime: _,
        files_from: _,
        substitutions: _,
        transforms: _,
        same_owner: _,
        no_same_owner: _,
        working_dir: _,
        allow_unsafe_links: _,
        file,
        files,
        null: _,
        ..
    } = args;

    let target = match file {
        Some(name) if name != "-" => PathBuf::from(name),
        Some(_) => {
            bail!("--append-to/-A requires a real archive file; stdin/stdout is not supported")
        }
        None => bail!("--append-to/-A requires --file <ARCHIVE> to be specified"),
    };

    if files.is_empty() {
        bail!("--append-to/-A expects at least one source archive to concatenate");
    }

    let sources = files.into_iter().map(PathBuf::from).collect();

    run_concat_from_stdio(ConcatFromStdioArgs {
        overwrite,
        target,
        sources,
    })
}

fn run_delete(args: StdioCommand) -> anyhow::Result<()> {
    let StdioCommand {
        create: _,
        extract: _,
        list: _,
        append: _,
        append_to: _,
        update: _,
        delete: _,
        recursive: _,
        no_recursive: _,
        overwrite: _,
        keep_dir: _,
        no_keep_dir: _,
        keep_timestamp: _,
        keep_permission: _,
        keep_xattr: _,
        keep_acl: _,
        solid: _,
        compression: _,
        cipher: _,
        hash: _,
        password,
        include,
        exclude,
        exclude_from,
        exclude_vcs,
        gitignore: _,
        follow_links: _,
        follow_command_links: _,
        out_dir: _,
        strip_components: _,
        uname: _,
        gname: _,
        uid: _,
        gid: _,
        numeric_owner: _,
        ctime: _,
        clamp_ctime: _,
        atime: _,
        clamp_atime: _,
        mtime: _,
        clamp_mtime: _,
        files_from,
        substitutions: _,
        transforms: _,
        same_owner: _,
        no_same_owner: _,
        working_dir: _,
        allow_unsafe_links: _,
        file,
        files,
        null,
        ..
    } = args;

    let archive = match file {
        Some(name) if name != "-" => PathBuf::from(name),
        Some(_) => bail!("--delete/-d requires a real archive file; stdin/stdout is not supported"),
        None => bail!("--delete/-d requires --file <ARCHIVE> to be specified"),
    };

    let delete_args = DeleteFromStdioArgs {
        output: None,
        files_from,
        files_from_stdin: false,
        include,
        exclude,
        exclude_from: exclude_from.map(PathBuf::from),
        exclude_vcs,
        null,
        password,
        transform_strategy: SolidEntriesTransformStrategyArgs {
            unsolid: false,
            keep_solid: false,
        },
        file: FileArgs { archive, files },
    };

    run_delete_from_stdio(delete_args)
}

fn run_update(args: StdioCommand) -> anyhow::Result<()> {
    let StdioCommand {
        create: _,
        extract: _,
        list: _,
        append: _,
        update: _,
        delete: _,
        recursive,
        no_recursive,
        overwrite: _,
        keep_dir,
        no_keep_dir,
        keep_timestamp,
        keep_permission,
        keep_xattr,
        keep_acl,
        solid: _,
        compression,
        cipher,
        hash,
        password,
        include,
        exclude,
        exclude_from,
        exclude_vcs,
        gitignore,
        follow_links,
        follow_command_links,
        check_links,
        one_file_system,
        out_dir: _,
        strip_components: _,
        uname,
        gname,
        uid,
        gid,
        numeric_owner,
        ctime,
        clamp_ctime,
        atime,
        clamp_atime,
        mtime,
        clamp_mtime,
        files_from,
        substitutions,
        transforms,
        same_owner: _,
        no_same_owner: _,
        working_dir,
        allow_unsafe_links: _,
        file,
        files,
        null,
        ..
    } = args;

    let archive = match file {
        Some(name) if name != "-" => PathBuf::from(name),
        Some(_) => {
            bail!("--update/-u requires a real archive file; stdin/stdout is not supported")
        }
        None => bail!("--update/-u requires --file <ARCHIVE> to be specified"),
    };

    run_update_from_stdio(UpdateFromStdioArgs {
        recursive,
        no_recursive,
        keep_dir,
        no_keep_dir,
        keep_timestamp,
        keep_permission,
        keep_xattr,
        keep_acl,
        uname,
        gname,
        uid,
        gid,
        numeric_owner,
        ctime,
        clamp_ctime,
        atime,
        clamp_atime,
        mtime,
        clamp_mtime,
        older_ctime: None,
        older_mtime: None,
        newer_ctime: None,
        newer_mtime: None,
        files_from,
        files_from_stdin: false,
        include,
        exclude,
        exclude_from,
        exclude_vcs,
        substitutions,
        transforms,
        working_dir,
        compression,
        password,
        cipher,
        hash,
        transform_strategy: SolidEntriesTransformStrategyArgs {
            unsolid: false,
            keep_solid: false,
        },
        file: FileArgs { archive, files },
        null,
        gitignore,
        follow_links,
        follow_command_links,
        check_links,
        one_file_system,
    })
}

fn run_append(args: StdioCommand) -> anyhow::Result<()> {
    let current_dir = env::current_dir()?;
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
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
    let create_options = CreateOptions {
        option,
        keep_options,
        owner_options,
        time_options,
    };
    let path_transformers = PathTransformers::new(args.substitutions, args.transforms);

    // NOTE: "-" will use stdin/out
    let mut file = args.file;
    file.take_if(|it| it == "-");
    let archive_path = file.take().map(|p| current_dir.join(p));
    let mut files = args.files;
    if let Some(path) = args.files_from {
        files.extend(read_paths(path, args.null)?);
    }
    let mut archive_sources_raw = Vec::new();
    files.retain(|entry| {
        if let Some(rest) = entry.strip_prefix('@') {
            archive_sources_raw.push(rest.to_string());
            false
        } else {
            true
        }
    });
    let exclude = {
        let mut exclude = args.exclude.unwrap_or_default();
        if let Some(p) = args.exclude_from {
            exclude.extend(read_paths(p, args.null)?);
        }
        if args.exclude_vcs {
            exclude.extend(VCS_FILES.iter().map(|it| String::from(*it)))
        }
        Exclude {
            include: args.include.unwrap_or_default().into(),
            exclude: exclude.into(),
        }
    };

    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
    }
    let base_dir = env::current_dir()?;
    let archive_sources = archive_sources_raw
        .into_iter()
        .map(PathBuf::from)
        .map(|src| {
            if src.is_absolute() {
                src
            } else {
                base_dir.join(src)
            }
        })
        .collect::<Vec<_>>();
    if let Some(file) = &archive_path {
        let archive = open_archive_then_seek_to_end(file)?;
        let target_items = collect_items(
            &files,
            args.recursive,
            args.keep_dir,
            args.gitignore,
            args.follow_links,
            args.follow_command_links,
            args.one_file_system,
            &exclude,
        )?;
        if args.check_links {
            ensure_hardlinks_complete(&target_items, args.follow_links)?;
        }
        run_append_archive(&create_options, &path_transformers, archive, target_items)?;
        if !archive_sources.is_empty() {
            append_archives_into_existing(file, &archive_sources)?;
        }
        Ok(())
    } else {
        if !archive_sources.is_empty() {
            bail!("@archive inputs are not supported when using stdin/stdout append mode");
        }
        let target_items = collect_items(
            &files,
            args.recursive,
            args.keep_dir,
            args.gitignore,
            args.follow_links,
            args.follow_command_links,
            args.one_file_system,
            &exclude,
        )?;
        if args.check_links {
            ensure_hardlinks_complete(&target_items, args.follow_links)?;
        }
        let mut output_archive = Archive::write_header(io::stdout().lock())?;
        {
            let mut input_archive = Archive::read_header(io::stdin().lock())?;
            for entry in input_archive.raw_entries() {
                output_archive.add_entry(entry?)?;
            }
        }
        run_append_archive(
            &create_options,
            &path_transformers,
            output_archive,
            target_items,
        )
    }
}

fn apply_auto_compress(compression: &mut CompressionAlgorithmArgs, archive_path: &Path) {
    let name = archive_path.to_string_lossy().to_ascii_lowercase();
    compression.store = false;
    compression.deflate = None;
    compression.zstd = None;
    compression.xz = None;

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".taz") {
        compression.deflate = Some(None);
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        compression.xz = Some(None);
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        compression.zstd = Some(None);
    } else if name.ends_with(".tar") {
        compression.store = true;
    } else {
        compression.zstd = Some(None);
    }
}
