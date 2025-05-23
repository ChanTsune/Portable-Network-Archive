use crate::{
    cli::{
        CipherAlgorithmArgs, CompressionAlgorithmArgs, FileArgs, HashAlgorithmArgs, PasswordArgs,
    },
    command::{
        ask_password, check_password,
        commons::{
            collect_items, create_entry, entry_option, CreateOptions, KeepOptions, OwnerOptions,
            PathTransformers,
        },
        Command,
    },
    utils::{
        self,
        re::{bsd::SubstitutionRule, gnu::TransformRule},
        PathPartExt,
    },
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::Archive;
use std::{fs::File, io, path::PathBuf};

#[derive(Parser, Clone, Debug)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["keep_acl"]).requires("unstable")),
    group(ArgGroup::new("unstable-append-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from-stdin").args(["files_from_stdin"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-from").args(["exclude_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-gitignore").args(["gitignore"]).requires("unstable")),
    group(ArgGroup::new("unstable-substitution").args(["substitutions"]).requires("unstable")),
    group(ArgGroup::new("unstable-transform").args(["transforms"]).requires("unstable")),
    group(ArgGroup::new("path-transform").args(["substitutions", "transforms"])),
    group(ArgGroup::new("read-files-from").args(["files_from", "files_from_stdin"])),
    group(ArgGroup::new("store-uname").args(["uname"]).requires("keep_permission")),
    group(ArgGroup::new("store-gname").args(["gname"]).requires("keep_permission")),
    group(ArgGroup::new("store-numeric-owner").args(["numeric_owner"]).requires("keep_permission")),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission"]).requires("unstable")),
))]
pub(crate) struct AppendCommand {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Archiving the directories")]
    pub(crate) keep_dir: bool,
    #[arg(
        long,
        visible_alias = "preserve-timestamps",
        help = "Archiving the timestamp of the files"
    )]
    pub(crate) keep_timestamp: bool,
    #[arg(
        long,
        visible_alias = "preserve-permissions",
        help = "Archiving the permissions of the files"
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
        help = "Archiving the acl of the files"
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
    #[arg(long, help = "Read archiving files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) files_from: Option<String>,
    #[arg(long, help = "Read archiving files from stdin (unstable)")]
    pub(crate) files_from_stdin: bool,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    pub(crate) exclude_from: Option<String>,
    #[arg(long, help = "Ignore files from .gitignore (unstable)")]
    pub(crate) gitignore: bool,
    #[arg(long, help = "Follow symbolic links")]
    pub(crate) follow_links: bool,
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
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) hash: HashAlgorithmArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    pub(crate) exclude: Option<Vec<PathBuf>>,
}

impl Command for AppendCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        append_to_archive(self)
    }
}

fn append_to_archive(args: AppendCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let archive_path = args.file.archive;
    if !archive_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not exists", archive_path.display()),
        ));
    }
    let mut num = 1;
    let file = File::options().write(true).read(true).open(&archive_path)?;
    let mut archive = Archive::read_header(file)?;
    let mut archive = loop {
        archive.seek_to_end()?;
        if !archive.has_next_archive() {
            break archive;
        }
        num += 1;
        let file = File::options()
            .write(true)
            .read(true)
            .open(archive_path.with_part(num).unwrap())?;
        archive = archive.read_next_archive(file)?;
    };

    let mut files = args.file.files;
    if args.files_from_stdin {
        files.extend(io::stdin().lines().collect::<io::Result<Vec<_>>>()?);
    } else if let Some(path) = args.files_from {
        files.extend(utils::fs::read_to_lines(path)?);
    }
    let exclude = {
        let mut exclude = Vec::new();
        if let Some(e) = args.exclude {
            exclude.extend(e);
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

    let (tx, rx) = std::sync::mpsc::channel();
    let password = password.as_deref();
    let option = entry_option(args.compression, args.cipher, args.hash, password);
    let keep_options = KeepOptions {
        keep_timestamp: args.keep_timestamp,
        keep_permission: args.keep_permission,
        keep_xattr: args.keep_xattr,
        keep_acl: args.keep_acl,
        // Defaults for append command (features not directly exposed for append)
        restore_windows_attributes: false,
        store_windows_attributes: false, // Or consider if it should inherit from archive or have specific flags
        store_windows_properties: false, // Or consider if it should inherit from archive or have specific flags
        restore_windows_properties: false,
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
    for file in target_items {
        let tx = tx.clone();
        rayon::scope_fifo(|s| {
            s.spawn_fifo(|_| {
                log::debug!("Adding: {}", file.display());
                tx.send(create_entry(&file, &create_options, &path_transformers))
                    .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
            })
        });
    }

    drop(tx);

    for entry in rx.into_iter() {
        archive.add_entry(entry?)?;
    }
    archive.finalize()?;
    Ok(())
}
