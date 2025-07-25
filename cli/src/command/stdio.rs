use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, DateTime, HashAlgorithmArgs, PasswordArgs,
    },
    command::{
        append::{open_archive_then_seek_to_end, run_append_archive},
        ask_password, check_password,
        commons::{
            collect_items, collect_split_archives, entry_option, read_paths, CreateOptions,
            Exclude, KeepOptions, OwnerOptions, PathTransformers, TimeOptions,
        },
        create::{create_archive_file, CreationContext},
        extract::{run_extract_archive_reader, OutputOption},
        list::{ListOptions, TimeField, TimeFormat},
        Command,
    },
    utils::{
        self,
        re::{bsd::SubstitutionRule, gnu::TransformRule},
        GlobPatterns, VCS_FILES,
    },
};
use clap::{ArgGroup, Args, ValueHint};
use pna::Archive;
use std::{env, io, path::PathBuf, time::SystemTime};

#[derive(Args, Clone, Debug)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["keep_acl"]).requires("unstable")),
    group(ArgGroup::new("bundled-flags").args(["create", "extract", "list"]).required(true)),
    group(ArgGroup::new("unstable-include").args(["include"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-from").args(["exclude_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(
        ArgGroup::new("from-input")
            .args(["files_from", "exclude_from"])
            .multiple(true)
    ),
    group(ArgGroup::new("null-requires").arg("null").requires("from-input")),
    group(ArgGroup::new("unstable-gitignore").args(["gitignore"]).requires("unstable")),
    group(ArgGroup::new("unstable-substitution").args(["substitutions"]).requires("unstable")),
    group(ArgGroup::new("unstable-transform").args(["transforms"]).requires("unstable")),
    group(ArgGroup::new("path-transform").args(["substitutions", "transforms"])),
    group(ArgGroup::new("owner-flag").args(["same_owner", "no_same_owner"])),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
    group(ArgGroup::new("recursive-flag").args(["recursive", "no_recursive"])),
    group(ArgGroup::new("keep-dir-flag").args(["keep_dir", "no_keep_dir"])),
    group(ArgGroup::new("action-flags").args(["create", "extract", "list", "append"])),
    group(ArgGroup::new("ctime-flag").args(["clamp_ctime"]).requires("ctime")),
    group(ArgGroup::new("mtime-flag").args(["clamp_mtime"]).requires("mtime")),
    group(ArgGroup::new("atime-flag").args(["clamp_atime"]).requires("atime")),
    group(ArgGroup::new("unstable-exclude-vcs").args(["exclude_vcs"]).requires("unstable")),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission"]).requires("unstable")),
))]
pub(crate) struct StdioCommand {
    #[arg(short, long, help = "Create archive")]
    create: bool,
    #[arg(short = 'x', long, help = "Extract archive")]
    extract: bool,
    #[arg(short = 't', long, help = "List files in archive")]
    list: bool,
    #[arg(long, help = "Append files to archive")]
    append: bool,
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
        long,
        visible_alias = "preserve-timestamps",
        help = "Archiving the timestamp of the files"
    )]
    keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Archiving the permissions of the files"
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
        help = "Archiving the acl of the files"
    )]
    keep_acl: bool,
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
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions"
    )]
    include: Option<Vec<String>>,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    pub(crate) exclude: Option<Vec<String>>,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    exclude_from: Option<String>,
    #[arg(long, help = "Exclude vcs files (unstable)")]
    exclude_vcs: bool,
    #[arg(long, help = "Ignore files from .gitignore (unstable)")]
    pub(crate) gitignore: bool,
    #[arg(long, visible_aliases = ["dereference"], help = "Follow symbolic links")]
    follow_links: bool,
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
    #[arg(long, help = "Read archiving files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) files_from: Option<String>,
    #[arg(
        short = 's',
        value_name = "PATTERN",
        help = "Modify file or archive member names according to pattern that like BSD tar -s option"
    )]
    substitutions: Option<Vec<SubstitutionRule>>,
    #[arg(
        long = "transform",
        visible_alias = "xform",
        value_name = "PATTERN",
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
        help = "changes the directory before adding the following files",
        value_hint = ValueHint::DirPath
    )]
    working_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Allow extract symlink and hardlink that contains root path or parent path"
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
    let target_items = collect_items(
        &files,
        !args.no_recursive,
        args.keep_dir,
        args.gitignore,
        args.follow_links,
        exclude,
    )?;

    let password = password.as_deref();
    let cli_option = entry_option(args.compression, args.cipher, args.hash, password);
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
        follow_links: args.follow_links,
        path_transformers,
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
        overwrite: args.overwrite,
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
        follow_links: args.follow_links,
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
    if let Some(file) = &archive_path {
        let archive = open_archive_then_seek_to_end(file)?;
        let target_items = collect_items(
            &files,
            args.recursive,
            args.keep_dir,
            args.gitignore,
            args.follow_links,
            exclude,
        )?;
        run_append_archive(&create_options, &path_transformers, archive, target_items)
    } else {
        let target_items = collect_items(
            &files,
            args.recursive,
            args.keep_dir,
            args.gitignore,
            args.follow_links,
            exclude,
        )?;
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
