use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, HashAlgorithmArgs, PasswordArgs},
    command::{
        append::{open_archive_then_seek_to_end, run_append_archive},
        ask_password, check_password,
        commons::{
            collect_items, collect_split_archives, entry_option, CreateOptions, KeepOptions,
            OwnerOptions, PathTransformers,
        },
        create::create_archive_file,
        extract::{run_extract_archive_reader, OutputOption},
        list::{ListOptions, TimeField, TimeFormat},
        Command,
    },
    utils::{
        self,
        re::{bsd::SubstitutionRule, gnu::TransformRule},
        GlobPatterns,
    },
};
use clap::{ArgGroup, Args, Parser, ValueHint};
use pna::Archive;
use std::{fs, io, path::PathBuf, time::SystemTime};

#[derive(Args, Clone, Debug)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["keep_acl"]).requires("unstable")),
    group(ArgGroup::new("bundled-flags").args(["create", "extract", "list"]).required(true)),
    group(ArgGroup::new("unstable-exclude-from").args(["exclude_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-gitignore").args(["gitignore"]).requires("unstable")),
    group(ArgGroup::new("unstable-substitution").args(["substitutions"]).requires("unstable")),
    group(ArgGroup::new("unstable-transform").args(["transforms"]).requires("unstable")),
    group(ArgGroup::new("path-transform").args(["substitutions", "transforms"])),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
    group(ArgGroup::new("action-flags").args(["create", "extract", "list", "append"])),
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
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[arg(long, help = "Archiving the directories")]
    keep_dir: bool,
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
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    pub(crate) exclude: Option<Vec<String>>,
    #[arg(long, help = "Ignore files from .gitignore (unstable)")]
    pub(crate) gitignore: bool,
    #[arg(long, help = "Follow symbolic links")]
    pub(crate) follow_links: bool,
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
    #[arg(long, help = "Read archiving files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) files_from: Option<String>,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) exclude_from: Option<String>,
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
    #[arg(short, long, help = "Input archive file path")]
    file: Option<PathBuf>,
    #[arg(help = "Files or patterns")]
    files: Vec<String>,
}

impl Command for StdioCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        run_stdio(self)
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct FileArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    pub(crate) files: Vec<PathBuf>,
}

fn run_stdio(args: StdioCommand) -> io::Result<()> {
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

fn run_create_archive(args: StdioCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let mut files = args.files;
    if let Some(path) = args.files_from {
        files.extend(utils::fs::read_to_lines(path)?);
    }
    let exclude = {
        let mut exclude = Vec::new();
        if let Some(e) = args.exclude {
            exclude.extend(e.into_iter().map(PathBuf::from));
        }
        if let Some(p) = args.exclude_from {
            exclude.extend(utils::fs::read_to_lines(p)?.into_iter().map(PathBuf::from));
        }
        exclude
    };
    let target_items = collect_items(
        &files,
        args.recursive,
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
    if let Some(file) = args.file {
        create_archive_file(
            || fs::File::open(&file),
            cli_option,
            keep_options,
            owner_options,
            args.solid,
            path_transformers,
            target_items,
        )
    } else {
        create_archive_file(
            || Ok(io::stdout().lock()),
            cli_option,
            keep_options,
            owner_options,
            args.solid,
            path_transformers,
            target_items,
        )
    }
}

fn run_extract_archive(args: StdioCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;

    let exclude = {
        let mut exclude = Vec::new();
        if let Some(e) = args.exclude {
            exclude.extend(e);
        }
        if let Some(p) = args.exclude_from {
            exclude.extend(utils::fs::read_to_lines(p)?);
        }
        GlobPatterns::new(exclude)
    }
    .map_err(io::Error::other)?;

    let out_option = OutputOption {
        overwrite: args.overwrite,
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
        path_transformers: PathTransformers::new(args.substitutions, args.transforms),
    };
    if let Some(path) = args.file {
        let archives = collect_split_archives(&path)?;
        run_extract_archive_reader(archives, args.files, || password.as_deref(), out_option)
    } else {
        run_extract_archive_reader(
            std::iter::repeat_with(|| io::stdin().lock()),
            args.files,
            || password.as_deref(),
            out_option,
        )
    }
}

fn run_list_archive(args: StdioCommand) -> io::Result<()> {
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
    if let Some(path) = args.file {
        let archives = collect_split_archives(&path)?;
        crate::command::list::run_list_archive(
            archives,
            password.as_deref(),
            &args.files,
            list_options,
        )
    } else {
        crate::command::list::run_list_archive(
            std::iter::repeat_with(|| io::stdin().lock()),
            password.as_deref(),
            &args.files,
            list_options,
        )
    }
}

fn run_append(args: StdioCommand) -> io::Result<()> {
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
    let create_options = CreateOptions {
        option,
        keep_options,
        owner_options,
    };
    let path_transformers = PathTransformers::new(args.substitutions, args.transforms);

    let mut files = args.files;
    if let Some(path) = args.files_from {
        files.extend(utils::fs::read_to_lines(path)?);
    }
    let exclude = {
        let mut exclude = Vec::new();
        if let Some(e) = args.exclude {
            exclude.extend(e.into_iter().map(PathBuf::from));
        }
        if let Some(p) = args.exclude_from {
            exclude.extend(utils::fs::read_to_lines(p)?.into_iter().map(PathBuf::from));
        }
        exclude
    };

    if let Some(file) = &args.file {
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
