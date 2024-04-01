use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, PasswordArgs, Verbosity},
    command::{
        ask_password, check_password,
        commons::{collect_items, create_entry, entry_option},
        stdio::FileArgs,
        Command,
    },
};
use clap::Args;
use pna::{Archive, SolidEntryBuilder, WriteOption};
use rayon::ThreadPoolBuilder;
use std::io::{self, stdout};

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct CreateCommand {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Archiving the directories")]
    pub(crate) keep_dir: bool,
    #[arg(long, help = "Archiving the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Archiving the permissions of the files")]
    pub(crate) keep_permission: bool,
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
}

impl Command for CreateCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        create_archive(self, verbosity)
    }
}

fn create_archive(args: CreateCommand, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let target_items = collect_items(args.file.files, args.recursive, args.keep_dir)?;

    let (tx, rx) = std::sync::mpsc::channel();
    let cli_option = entry_option(args.compression, args.cipher, password);
    let option = if args.solid {
        WriteOption::store()
    } else {
        cli_option.clone()
    };
    for file in target_items {
        let option = option.clone();
        let keep_timestamp = args.keep_timestamp;
        let keep_permission = args.keep_permission;
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            if verbosity == Verbosity::Verbose {
                eprintln!("Adding: {}", file.display());
            }
            tx.send(create_entry(&file, option, keep_timestamp, keep_permission))
                .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
        });
    }

    drop(tx);

    let file = stdout();
    if args.solid {
        let mut entries_builder = SolidEntryBuilder::new(cli_option)?;
        for entry in rx.into_iter() {
            entries_builder.add_entry(entry?)?;
        }
        let entries = entries_builder.build()?;
        let mut writer = Archive::write_header(file)?;
        writer.add_entry(entries)?;
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
