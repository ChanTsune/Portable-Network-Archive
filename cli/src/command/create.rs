use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, FileArgs, PasswordArgs, Verbosity},
    command::{
        ask_password, check_password,
        commons::{collect_items, create_entry, entry_option, write_split_archive, KeepOptions},
        Command,
    },
};
use bytesize::ByteSize;
use clap::{ArgGroup, Parser, ValueHint};
use indicatif::HumanDuration;
use pna::{Archive, SolidEntryBuilder, WriteOption};
use rayon::ThreadPoolBuilder;
use std::{
    fs::{self, File},
    io::{self, prelude::*},
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("unstable-create-exclude").args(["exclude"]).requires("unstable")))]
pub(crate) struct CreateCommand {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Archiving the directories")]
    pub(crate) keep_dir: bool,
    #[arg(long, help = "Archiving the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Archiving the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Archiving the extended attributes of the files")]
    pub(crate) keep_xattr: bool,
    #[arg(long, help = "Split archive by total entry size")]
    pub(crate) split: Option<Option<ByteSize>>,
    #[arg(long, help = "Solid mode archive")]
    pub(crate) solid: bool,
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    pub(crate) exclude: Option<Vec<PathBuf>>,
}

impl Command for CreateCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        create_archive(self, verbosity)
    }
}

fn create_archive(args: CreateCommand, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password.clone())?;
    check_password(&password, &args.cipher);
    let start = Instant::now();
    let archive = &args.file.archive;
    if !args.overwrite && archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is already exists", archive.display()),
        ));
    }
    if verbosity != Verbosity::Quite {
        eprintln!("Create an archive: {}", archive.display());
    }

    let target_items = collect_items(
        &args.file.files,
        args.recursive,
        args.keep_dir,
        &args.exclude,
    )?;

    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
    }
    let max_file_size = args
        .split
        .map(|it| it.unwrap_or(ByteSize::gb(1)).0 as usize);

    let keep_options = KeepOptions {
        keep_timestamp: args.keep_timestamp,
        keep_permission: args.keep_permission,
        keep_xattr: args.keep_xattr,
    };
    let write_option = entry_option(args.compression, args.cipher, password);
    if let Some(size) = max_file_size {
        create_archive_with_split(
            &args.file.archive,
            write_option,
            keep_options,
            args.solid,
            target_items,
            size,
            verbosity,
        )?;
    } else {
        create_archive_file(
            || File::create(&args.file.archive),
            write_option,
            keep_options,
            args.solid,
            target_items,
            verbosity,
        )?;
    }
    if verbosity != Verbosity::Quite {
        eprintln!(
            "Successfully created an archive in {}",
            HumanDuration(start.elapsed())
        );
    }
    Ok(())
}

pub(crate) fn create_archive_file<W, F>(
    mut get_writer: F,
    write_option: WriteOption,
    keep_options: KeepOptions,
    solid: bool,
    target_items: Vec<PathBuf>,
    verbosity: Verbosity,
) -> io::Result<()>
where
    W: Write,
    F: FnMut() -> io::Result<W>,
{
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let (tx, rx) = std::sync::mpsc::channel();
    let option = if solid {
        WriteOption::store()
    } else {
        write_option.clone()
    };
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

    let file = get_writer()?;
    if solid {
        let mut writer = Archive::write_solid_header(file, write_option)?;
        for entry in rx.into_iter() {
            writer.add_entry(entry?)?;
        }
        writer.finalize()?;
    } else {
        let mut writer = Archive::write_header(file)?;
        for entry in rx.into_iter() {
            writer.add_entry(entry?)?;
        }
        writer.finalize()?;
    }
    Ok(())
}

fn create_archive_with_split(
    archive: &Path,
    write_option: WriteOption,
    keep_options: KeepOptions,
    solid: bool,
    target_items: Vec<PathBuf>,
    max_file_size: usize,
    verbosity: Verbosity,
) -> io::Result<()> {
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let (tx, rx) = std::sync::mpsc::channel();
    let option = if solid {
        WriteOption::store()
    } else {
        write_option.clone()
    };
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

    if solid {
        let mut entries_builder = SolidEntryBuilder::new(write_option)?;
        for entry in rx.into_iter() {
            entries_builder.add_entry(entry?)?;
        }
        let entries = entries_builder.build();
        write_split_archive(archive, [entries].into_iter(), max_file_size)?;
    } else {
        write_split_archive(archive, rx.into_iter(), max_file_size)?;
    }
    Ok(())
}
