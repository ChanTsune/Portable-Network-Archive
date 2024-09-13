#[cfg(feature = "memmap")]
use crate::command::commons::run_entries_mem as run_entries;
#[cfg(not(feature = "memmap"))]
use crate::command::commons::run_process_archive_path as run_entries;
use crate::{
    cli::{FileArgs, PasswordArgs},
    command::{ask_password, Command},
    utils::{self, GlobPatterns, PathPartExt},
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::Archive;
use std::{env::temp_dir, fs, io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(group(ArgGroup::new("unstable-delete-exclude").args(["exclude"]).requires("unstable")))]
pub(crate) struct DeleteCommand {
    #[arg(long, help = "Output file path", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    pub(crate) exclude: Option<Vec<globset::Glob>>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    file: FileArgs,
}

impl Command for DeleteCommand {
    fn execute(self) -> io::Result<()> {
        delete_file_from_archive(self)
    }
}

fn delete_file_from_archive(args: DeleteCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let globs = GlobPatterns::new(args.file.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let exclude_globs = GlobPatterns::try_from(args.exclude.unwrap_or_default())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let outfile_path = if let Some(output) = &args.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        output.clone()
    } else {
        let random = rand::random::<usize>();
        temp_dir().join(format!("{}.pna.tmp", random))
    };
    let outfile = fs::File::create(&outfile_path)?;
    let mut out_archive = Archive::write_header(outfile)?;

    run_entries(
        &args.file.archive,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let entry_path = entry.header().path().as_ref();
            if globs.matches_any(entry_path) && !exclude_globs.matches_any(entry_path) {
                return Ok(());
            }
            out_archive.add_entry(entry)?;
            Ok(())
        },
    )?;
    out_archive.finalize()?;

    if args.output.is_none() {
        utils::fs::mv(outfile_path, args.file.archive.remove_part().unwrap())?;
    }
    Ok(())
}
