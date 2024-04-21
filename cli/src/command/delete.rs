use crate::{
    cli::{FileArgs, PasswordArgs, Verbosity},
    command::{ask_password, Command},
    utils,
};
use clap::{Parser, ValueHint};
use pna::Archive;
use std::{env::temp_dir, fs, io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct DeleteCommand {
    #[arg(long, help = "Output file path", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    file: FileArgs,
}

impl Command for DeleteCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        delete_file_from_archive(self, verbosity)
    }
}

fn delete_file_from_archive(args: DeleteCommand, _verbosity: Verbosity) -> io::Result<()> {
    let outfile_path = if let Some(output) = &args.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        output.clone()
    } else {
        let random = rand::random::<usize>();
        temp_dir().join(format!("{}.pna.tmp", random))
    };
    let password = ask_password(args.password)?;
    let file = fs::File::open(&args.file.archive)?;
    let mut archive = Archive::read_header(file)?;
    let outfile = fs::File::create(&outfile_path)?;
    let mut out_archive = Archive::write_header(outfile)?;
    for entry in archive.entries_with_password(password.as_deref()) {
        let entry = entry?;
        if args
            .file
            .files
            .iter()
            .any(|d| entry.header().path().as_path().eq(d))
        {
            continue;
        }
        out_archive.add_entry(entry)?;
    }
    out_archive.finalize()?;

    if args.output.is_none() {
        utils::fs::mv(outfile_path, args.file.archive)?;
    }
    Ok(())
}
