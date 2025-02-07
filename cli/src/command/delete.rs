use crate::{
    cli::{
        FileArgs, PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs,
    },
    command::{
        ask_password,
        commons::{run_transform_entry, TransformStrategyKeepSolid, TransformStrategyUnSolid},
        Command,
    },
    utils::{self, GlobPatterns, PathPartExt},
};
use clap::{ArgGroup, Parser, ValueHint};
use std::{io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from-stdin").args(["files_from_stdin"]).requires("unstable")),
    group(ArgGroup::new("unstable-delete-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-from").args(["exclude_from"]).requires("unstable")),
    group(ArgGroup::new("read-files-from").args(["files_from", "files_from_stdin"])),
)]
pub(crate) struct DeleteCommand {
    #[arg(long, help = "Output file path", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    #[arg(long, help = "Read deleting files from given path (unstable)", value_hint = ValueHint::FilePath)]
    files_from: Option<String>,
    #[arg(long, help = "Read deleting files from stdin (unstable)")]
    files_from_stdin: bool,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    exclude: Option<Vec<String>>,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    exclude_from: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    file: FileArgs,
}

impl Command for DeleteCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        delete_file_from_archive(self)
    }
}

fn delete_file_from_archive(args: DeleteCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let mut files = args.file.files;
    if args.files_from_stdin {
        files.extend(io::stdin().lines().collect::<io::Result<Vec<_>>>()?);
    } else if let Some(path) = args.files_from {
        files.extend(utils::fs::read_to_lines(path)?);
    }
    let globs =
        GlobPatterns::new(files).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let exclude_globs = {
        let mut exclude = args.exclude.unwrap_or_default();
        if let Some(p) = args.exclude_from {
            exclude.extend(utils::fs::read_to_lines(p)?);
        }
        GlobPatterns::new(exclude)
    }
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            args.output
                .unwrap_or_else(|| args.file.archive.remove_part().unwrap()),
            &args.file.archive,
            || password.as_deref(),
            |entry| {
                let entry = entry?;
                let entry_path = entry.header().path();
                if globs.matches_any(entry_path) && !exclude_globs.matches_any(entry_path) {
                    return Ok(None);
                }
                Ok(Some(entry))
            },
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            args.output
                .unwrap_or_else(|| args.file.archive.remove_part().unwrap()),
            &args.file.archive,
            || password.as_deref(),
            |entry| {
                let entry = entry?;
                let entry_path = entry.header().path();
                if globs.matches_any(entry_path) && !exclude_globs.matches_any(entry_path) {
                    return Ok(None);
                }
                Ok(Some(entry))
            },
            TransformStrategyKeepSolid,
        ),
    }
}
