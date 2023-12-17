use crate::{
    cli::{AppendArgs, Verbosity},
    command::{
        ask_password, check_password,
        commons::{collect_items, create_entry, entry_option},
        Command,
    },
};
use pna::Archive;
use rayon::ThreadPoolBuilder;
use std::{fs::File, io};

impl Command for AppendArgs {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        append_to_archive(self, verbosity)
    }
}

fn append_to_archive(args: AppendArgs, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let archive = args.file.archive;
    if !archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not exists", archive.display()),
        ));
    }
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let file = File::options().write(true).read(true).open(&archive)?;
    let mut archive = Archive::read_header(file)?;
    archive.seek_to_end()?;

    let target_items = collect_items(args.file.files, args.recursive, args.keep_dir)?;
    let (tx, rx) = std::sync::mpsc::channel();
    let option = entry_option(args.compression, args.cipher, password);
    for file in target_items {
        let option = option.clone();
        let keep_timestamp = args.keep_timestamp;
        let keep_permission = args.keep_permission;
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            if verbosity == Verbosity::Verbose {
                println!("Adding: {}", file.display());
            }
            tx.send(create_entry(&file, option, keep_timestamp, keep_permission))
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
