use crate::{
    cli::{FileArgs, PasswordArgs},
    command::{ask_password, commons::run_manipulate_entry, Command},
    utils::{GlobPatterns, PathPartExt},
};
use clap::{ArgGroup, Parser, ValueHint};
use std::{io, path::PathBuf};

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
    run_manipulate_entry(
        args.output
            .unwrap_or_else(|| args.file.archive.remove_part().unwrap()),
        &args.file.archive,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let entry_path = entry.header().path().as_ref();
            if globs.matches_any(entry_path) && !exclude_globs.matches_any(entry_path) {
                return Ok(None);
            }
            Ok(Some(entry))
        },
    )
}
