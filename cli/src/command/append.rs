use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, FileArgs, PasswordArgs, Verbosity},
    command::{
        ask_password, check_password,
        commons::{
            collect_items, create_entry, entry_option, CreateOptions, KeepOptions, OwnerOptions,
        },
        Command,
    },
    utils::{self, PathPartExt},
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::Archive;
use rayon::ThreadPoolBuilder;
use std::{fs::File, io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    group(ArgGroup::new("unstable-append-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from-stdin").args(["files_from_stdin"]).requires("unstable")),
    group(ArgGroup::new("read-files-from").args(["files_from", "files_from_stdin"])),
    group(ArgGroup::new("store-uname").args(["uname"]).requires("keep_permission")),
    group(ArgGroup::new("store-gname").args(["gname"]).requires("keep_permission")),
    group(ArgGroup::new("store-numeric-owner").args(["numeric_owner"]).requires("keep_permission")),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
)]
pub(crate) struct AppendCommand {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Archiving the directories")]
    pub(crate) keep_dir: bool,
    #[arg(long, help = "Archiving the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Archiving the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Archiving the extended attributes of the files")]
    pub(crate) keep_xattr: bool,
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
    #[arg(long, help = "Read archiving files from given path", value_hint = ValueHint::FilePath)]
    pub(crate) files_from: Option<String>,
    #[arg(long, help = "Read archiving files from stdin")]
    pub(crate) files_from_stdin: bool,
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    pub(crate) exclude: Option<Vec<PathBuf>>,
}

impl Command for AppendCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        append_to_archive(self, verbosity)
    }
}

fn append_to_archive(args: AppendCommand, verbosity: Verbosity) -> io::Result<()> {
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
        if !archive.next_archive() {
            break archive;
        }
        num += 1;
        let file = File::options()
            .write(true)
            .read(true)
            .open(archive_path.with_part(num).unwrap())?;
        archive = archive.read_next_archive(file)?;
    };
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut files = args.file.files;
    if args.files_from_stdin {
        files.extend(
            io::stdin()
                .lines()
                .map(|l| l.map(PathBuf::from))
                .collect::<io::Result<Vec<_>>>()?,
        );
    } else if let Some(path) = args.files_from {
        files.extend(
            utils::fs::read_to_lines(path)?
                .into_iter()
                .map(PathBuf::from),
        );
    }
    let target_items = collect_items(&files, args.recursive, args.keep_dir, &args.exclude)?;

    let (tx, rx) = std::sync::mpsc::channel();
    let option = entry_option(args.compression, args.cipher, password);
    let keep_options = KeepOptions {
        keep_timestamp: args.keep_timestamp,
        keep_permission: args.keep_permission,
        keep_xattr: args.keep_xattr,
    };
    let owner_options = OwnerOptions {
        uname: if args.numeric_owner {
            Some("".to_string())
        } else {
            args.uname
        },
        gname: if args.numeric_owner {
            Some("".to_string())
        } else {
            args.gname
        },
        uid: args.uid,
        gid: args.gid,
    };
    for file in target_items {
        let option = option.clone();
        let owner_options = owner_options.clone();
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            if verbosity == Verbosity::Verbose {
                eprintln!("Adding: {}", file.display());
            }
            let create_options = CreateOptions {
                option,
                keep_options,
                owner_options,
            };
            tx.send(create_entry(&file, create_options))
                .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
        });
    }

    drop(tx);

    for entry in rx.into_iter() {
        archive.add_entry(entry?)?;
    }
    archive.finalize()?;
    Ok(())
}
