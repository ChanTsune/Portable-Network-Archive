use crate::{
    cli::{PasswordArgs, Verbosity},
    command::{
        ask_password,
        commons::{run_manipulate_entry_by_path, run_process_archive_path},
        Command,
    },
    utils::{GlobPatterns, PathPartExt},
};
use clap::{Parser, ValueHint};
use indexmap::IndexMap;
use std::{io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct XattrCommand {
    #[command(subcommand)]
    command: XattrCommands,
}

impl Command for XattrCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        match self.command {
            XattrCommands::Get(cmd) => cmd.execute(verbosity),
            XattrCommands::Set(cmd) => cmd.execute(verbosity),
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum XattrCommands {
    #[command(about = "Get extended attributes of entries")]
    Get(GetXattrCommand),
    #[command(about = "Set extended attributes of entries")]
    Set(SetXattrCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct GetXattrCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[arg(short, long, help = "Filter by name of extended attribute")]
    name: Option<String>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for GetXattrCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        archive_get_xattr(self, verbosity)
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SetXattrCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[arg(short, long, help = "Name of extended attribute")]
    name: Option<String>,
    #[arg(short, long, help = "Value of extended attribute")]
    value: Option<String>,
    #[arg(short = 'x', long, help = "Remove extended attribute")]
    remove: Option<String>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SetXattrCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        archive_set_xattr(self, verbosity)
    }
}

fn archive_get_xattr(args: GetXattrCommand, _: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    run_process_archive_path(
        &args.archive,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path().as_ref();
            if globs.matches_any(name) {
                println!("{}", name);
                for attr in entry.xattrs().iter().filter(|a| {
                    args.name.is_none() || args.name.as_deref().is_some_and(|it| it == a.name())
                }) {
                    println!(
                        "{}: {}",
                        attr.name(),
                        String::from_utf8(attr.value().into()).unwrap_or_else(|e| e.to_string())
                    );
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}

fn archive_set_xattr(args: SetXattrCommand, _: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    run_manipulate_entry_by_path(
        args.archive.remove_part().unwrap(),
        &args.archive,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path().as_ref();
            if globs.matches_any(name) {
                let mut xattrs = entry
                    .xattrs()
                    .iter()
                    .map(|it| (it.name(), it.value()))
                    .collect::<IndexMap<_, _>>();
                if let Some(name) = args.name.as_deref() {
                    let map_entry = xattrs.entry(name);
                    map_entry.or_insert(args.value.as_deref().unwrap_or_default().as_bytes());
                }
                if let Some(name) = args.name.as_deref() {
                    xattrs.shift_remove_entry(name);
                }
                let xattrs = xattrs
                    .into_iter()
                    .map(|(key, value)| pna::ExtendedAttribute::new(key.into(), value.into()))
                    .collect::<Vec<_>>();
                Ok(entry.with_xattrs(&xattrs))
            } else {
                Ok(entry)
            }
        },
    )
}