use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, DateTime, FileArgsCompat, HashAlgorithmArgs,
        PasswordArgs,
    },
    command::{
        ask_password, check_password,
        commons::{
            collect_items, create_entry, entry_option, read_paths, read_paths_stdin, CreateOptions,
            Exclude, KeepOptions, OwnerOptions, PathTransformers, StoreAs, TimeOptions,
        },
        Command,
    },
    utils::{
        re::{bsd::SubstitutionRule, gnu::TransformRule},
        PathPartExt, VCS_FILES,
    },
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::Archive;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

#[derive(Parser, Clone, Debug)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["keep_acl"]).requires("unstable")),
    group(ArgGroup::new("include-group").args(["include"])),
    group(ArgGroup::new("append-exclude-group").args(["exclude"])),
    group(ArgGroup::new("files-from-group").args(["files_from"])),
    group(ArgGroup::new("files-from-stdin-group").args(["files_from_stdin"])),
    group(ArgGroup::new("exclude-from-group").args(["exclude_from"])),
    group(ArgGroup::new("gitignore-group").args(["gitignore"])),
    group(ArgGroup::new("substitution-group").args(["substitutions"])),
    group(ArgGroup::new("transform-group").args(["transforms"])),
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
    group(ArgGroup::new("mtime-flag").args(["clamp_mtime"]).requires("mtime")),
    group(ArgGroup::new("atime-flag").args(["clamp_atime"]).requires("atime")),
    group(ArgGroup::new("exclude-vcs-group").args(["exclude_vcs"])),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission"]).requires("unstable")),
))]
pub(crate) struct AppendCommand {
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
    pub(crate) keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Archiving the permissions of the files (unstable on Windows)"
    )]
    pub(crate) keep_permission: bool,
    #[arg(
        long,
        visible_alias = "preserve-xattrs",
        help = "Archiving the extended attributes of the files"
    )]
    pub(crate) keep_xattr: bool,
    #[arg(
        long,
        visible_alias = "preserve-acls",
        help = "Archiving the acl of the files (unstable)"
    )]
    pub(crate) keep_acl: bool,
    #[arg(long, help = "Archiving user to the entries from given name")]
    pub(crate) uname: Option<String>,
    #[arg(long, help = "Archiving group to the entries from given name")]
    pub(crate) gname: Option<String>,
    #[arg(
        long,
        help = "Overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id"
    )]
    pub(crate) uid: Option<u32>,
    #[arg(
        long,
        help = "Overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id"
    )]
    pub(crate) gid: Option<u32>,
    #[arg(
        long,
        help = "This is equivalent to --uname \"\" --gname \"\". It causes user and group names to not be stored in the archive"
    )]
    pub(crate) numeric_owner: bool,
    #[arg(long, help = "Overrides the creation time read from disk")]
    ctime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the creation time of the entries to the specified time by --ctime"
    )]
    clamp_ctime: bool,
    #[arg(long, help = "Overrides the access time read from disk")]
    atime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the access time of the entries to the specified time by --atime"
    )]
    clamp_atime: bool,
    #[arg(long, help = "Overrides the modification time read from disk")]
    mtime: Option<DateTime>,
    #[arg(
        long,
        help = "Clamp the modification time of the entries to the specified time by --mtime"
    )]
    clamp_mtime: bool,
    #[arg(
        short = 'T',
        long,
        help = "Read archive entries from file",
        value_hint = ValueHint::FilePath
    )]
    pub(crate) files_from: Option<String>,
    #[arg(long, help = "Read archiving files from stdin (unstable)")]
    pub(crate) files_from_stdin: bool,
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
    exclude: Option<Vec<String>>,
    #[arg(
        short = 'X',
        long,
        help = "Read exclude patterns from file",
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
    #[arg(
        long = "one-file-system",
        help = "When recursing, stay on the same file system as the source path"
    )]
    one_file_system: bool,
    #[arg(
        long,
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
    #[arg(
        long = "nodump",
        help = "Exclude files or directories marked with the nodump flag"
    )]
    nodump: bool,
    #[arg(
        short = 's',
        value_name = "PATTERN",
        help = "Modify file or archive member names according to the BSD tar -s pattern rules"
    )]
    substitutions: Option<Vec<SubstitutionRule>>,
    #[arg(
        long = "transform",
        visible_alias = "xform",
        value_name = "PATTERN",
        help = "Modify file or archive member names using GNU tar --transform pattern rules"
    )]
    transforms: Option<Vec<TransformRule>>,
    #[arg(
        short = 'C',
        long = "cd",
        visible_aliases = ["directory"],
        value_name = "DIRECTORY",
        help = "changes the directory before adding the following files",
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
    pub(crate) file: FileArgsCompat,
}

impl Command for AppendCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        append_to_archive(self)
    }
}

fn append_to_archive(args: AppendCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let archive_path = args.file.archive();
    if !archive_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not exists", archive_path.display()),
        )
        .into());
    }
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

    let archive = open_archive_then_seek_to_end(&archive_path)?;

    let mut files = args.file.files();
    if args.files_from_stdin {
        files.extend(read_paths_stdin(args.null)?);
    } else if let Some(path) = args.files_from {
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
        args.follow_command_links,
        args.one_file_system,
        args.nodump,
        None,
        &exclude,
    )?;

    run_append_archive(&create_options, &path_transformers, archive, target_items)
}

pub(crate) fn run_append_archive(
    create_options: &CreateOptions,
    path_transformers: &Option<PathTransformers>,
    mut archive: Archive<impl io::Write>,
    target_items: Vec<(PathBuf, StoreAs)>,
) -> anyhow::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    rayon::scope_fifo(|s| {
        for file in target_items {
            let tx = tx.clone();
            s.spawn_fifo(move |_| {
                log::debug!("Adding: {}", file.0.display());
                tx.send(create_entry(&file, create_options, path_transformers))
                    .unwrap_or_else(|e| log::error!("{e}: {}", file.0.display()));
            })
        }

        drop(tx);
    });

    for entry in rx.into_iter() {
        archive.add_entry(entry?)?;
    }
    archive.finalize()?;
    Ok(())
}

pub(crate) fn open_archive_then_seek_to_end(
    path: impl AsRef<Path>,
) -> anyhow::Result<Archive<fs::File>> {
    let archive_path = path.as_ref();
    let mut num = 1;
    let file = fs::File::options()
        .write(true)
        .read(true)
        .open(archive_path)?;
    let mut archive = Archive::read_header(file)?;
    loop {
        archive.seek_to_end()?;
        if !archive.has_next_archive() {
            break Ok(archive);
        }
        num += 1;
        let file = fs::File::options()
            .write(true)
            .read(true)
            .open(archive_path.with_part(num).unwrap())?;
        archive = archive.read_next_archive(file)?;
    }
}
