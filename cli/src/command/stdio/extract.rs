use crate::{
    cli::{PasswordArgs, Verbosity},
    command::{
        ask_password, commons::KeepOptions, extract::extract_entry, stdio::FileArgs, Command,
    },
    utils::GlobPatterns,
};
use clap::{Args, ValueHint};
use pna::{Archive, DataKind};
use rayon::ThreadPoolBuilder;
use std::{
    io::{self, stdin},
    path::PathBuf,
};

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct ExtractCommand {
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Output directory of extracted files", value_hint = ValueHint::DirPath)]
    pub(crate) out_dir: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[arg(long, help = "Restore the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Restore the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Restore the extended attributes of the files")]
    pub(crate) keep_xattr: bool,
    #[command(flatten)]
    pub(crate) file: FileArgs,
}

impl Command for ExtractCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        extract_archive(self, verbosity)
    }
}

fn extract_archive(args: ExtractCommand, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let keep_options = KeepOptions {
        keep_timestamp: args.keep_timestamp,
        keep_permission: args.keep_permission,
        keep_xattr: args.keep_xattr,
    };
    let globs = GlobPatterns::new(args.file.files.iter().map(|p| p.to_string_lossy()))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut hard_link_entries = Vec::new();

    let (tx, rx) = std::sync::mpsc::channel();
    let mut archive = Archive::read_header(stdin().lock())?;
    for entry in archive.entries_with_password(password.as_deref()) {
        let item = entry?;
        let item_path = PathBuf::from(item.header().path().as_str());
        if !globs.is_empty() && !globs.matches_any_path(&item_path) {
            if verbosity == Verbosity::Verbose {
                eprintln!("Skip: {}", item.header().path())
            }
            return Ok(());
        }
        if item.header().data_kind() == DataKind::HardLink {
            hard_link_entries.push(item);
            return Ok(());
        }
        let tx = tx.clone();
        let password = password.clone();
        let out_dir = args.out_dir.clone();
        pool.spawn_fifo(move || {
            tx.send(extract_entry(
                item,
                password,
                args.overwrite,
                out_dir.as_deref(),
                keep_options,
                verbosity,
            ))
            .unwrap_or_else(|e| panic!("{e}: {}", item_path.display()));
        });
    }
    drop(tx);
    for result in rx {
        result?;
    }

    for item in hard_link_entries {
        extract_entry(
            item,
            password.clone(),
            args.overwrite,
            args.out_dir.as_deref(),
            keep_options,
            verbosity,
        )?;
    }
    Ok(())
}
