use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, FileArgs, PasswordArgs, Verbosity},
    command::{
        ask_password, check_password,
        commons::{collect_items, create_entry, entry_option, run_process_archive, KeepOptions},
        Command,
    },
    utils,
    utils::remove_part_name,
};
use clap::{ArgGroup, Parser, ValueHint};
use normalize_path::*;
use pna::{Archive, RegularEntry};
use rayon::ThreadPoolBuilder;
use std::{
    env::temp_dir,
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("unstable-append-exclude").args(["exclude"]).requires("unstable")))]
pub(crate) struct UpdateCommand {
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
    #[arg(
        long,
        help = "Only include files and directories newer than the specified date. This compares ctime entries."
    )]
    pub(crate) newer_ctime: bool,
    #[arg(
        long,
        help = "Only include files and directories newer than the specified date. This compares mtime entries."
    )]
    pub(crate) newer_mtime: bool,
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

impl Command for UpdateCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        update_archive(self, verbosity)
    }
}

fn update_archive(args: UpdateCommand, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let archive_path = args.file.archive;
    if !archive_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not exists", archive_path.display()),
        ));
    }
    let option = entry_option(args.compression, args.cipher, password.clone());
    let keep_options = KeepOptions {
        keep_timestamp: args.keep_timestamp,
        keep_permission: args.keep_permission,
        keep_xattr: args.keep_xattr,
    };
    let mut target_items = collect_items(
        &args.file.files,
        args.recursive,
        args.keep_dir,
        &args.exclude,
    )?;

    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let (tx, rx) = std::sync::mpsc::channel();

    let random = rand::random::<usize>();
    let outfile_path = temp_dir().join(format!("{}.pna.tmp", random));

    let outfile = fs::File::create(&outfile_path)?;
    let mut out_archive = Archive::write_header(outfile)?;

    let need_update_condition = if args.newer_ctime {
        |path: &Path, entry: &RegularEntry| -> io::Result<bool> {
            let meta = fs::metadata(path)?;
            let ctime = meta.created()?;
            let d = entry.metadata().created().ok_or(io::ErrorKind::Other)?;
            Ok(SystemTime::UNIX_EPOCH + d < ctime)
        }
    } else if args.newer_mtime {
        |path: &Path, entry: &RegularEntry| -> io::Result<bool> {
            let meta = fs::metadata(path)?;
            let mtime = meta.modified()?;
            let d = entry.metadata().modified().ok_or(io::ErrorKind::Other)?;
            Ok(SystemTime::UNIX_EPOCH + d < mtime)
        }
    } else {
        |_: &Path, _: &RegularEntry| -> io::Result<bool> { Ok(true) }
    };

    run_process_archive(
        &archive_path,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let file = entry.header().path().as_path().to_path_buf();
            let normalized_path = file.normalize();
            if target_items.contains(&normalized_path) {
                if need_update_condition(&normalized_path, &entry).unwrap_or(true) {
                    let option = option.clone();
                    let tx = tx.clone();
                    pool.spawn_fifo(move || {
                        if verbosity == Verbosity::Verbose {
                            eprintln!("Updating: {}", file.display());
                        }
                        tx.send(create_entry(&file, option, keep_options))
                            .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
                    });
                } else {
                    out_archive.add_entry(entry)?;
                }
                target_items.retain(|p| p.normalize() == normalized_path);
                return Ok(());
            }
            Ok(())
        },
    )?;

    // NOTE: Add new entries
    for file in target_items {
        let option = option.clone();
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            if verbosity == Verbosity::Verbose {
                eprintln!("Adding: {}", file.display());
            }
            tx.send(create_entry(&file, option, keep_options))
                .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
        });
    }

    drop(tx);
    for entry in rx.into_iter() {
        out_archive.add_entry(entry?)?;
    }
    out_archive.finalize()?;

    utils::fs::mv(outfile_path, remove_part_name(&archive_path).unwrap())?;

    Ok(())
}
