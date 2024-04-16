use crate::{
    cli::{FileArgs, PasswordArgs, Verbosity},
    command::{ask_password, Command},
    utils::{self, part_name, remove_part_name},
};
use clap::{Args, ValueHint};
use pna::{Archive, Metadata};
use std::{env::temp_dir, fs, io, path::PathBuf};

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct StripCommand {
    #[arg(long, help = "Keep the directories")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Keep the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Keep the extended attributes of the files")]
    pub(crate) keep_xattr: bool,
    #[arg(long, help = "Output file path", value_hint = ValueHint::AnyPath)]
    pub(crate) output: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
}

impl Command for StripCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        strip_metadata(self, verbosity)
    }
}

fn strip_metadata(args: StripCommand, _verbosity: Verbosity) -> io::Result<()> {
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

    let mut num_archive = 1;
    loop {
        if archive.next_archive() {
            num_archive += 1;
            if let Ok(file) = fs::File::open(part_name(&args.file.archive, num_archive).unwrap()) {
                archive = archive.read_next_archive(file)?;
            } else {
                eprintln!("Detected that the file has been split, but the following file could not be found.");
                break;
            }
        } else {
            break;
        }
    }

    for entry in archive.entries_with_password(password.as_deref()) {
        let mut entry = entry?;
        let mut metadata = Metadata::new();
        if args.keep_permission {
            metadata = metadata.with_permission(entry.metadata().permission().cloned());
        }
        if args.keep_timestamp {
            metadata = metadata.with_accessed(entry.metadata().accessed());
            metadata = metadata.with_created(entry.metadata().created());
            metadata = metadata.with_modified(entry.metadata().modified());
        }
        entry = entry.with_metadata(metadata);
        if !args.keep_xattr {
            entry = entry.with_xattrs(&[]);
        }
        out_archive.add_entry(entry)?;
    }
    out_archive.finalize()?;

    if args.output.is_none() {
        utils::fs::mv(outfile_path, remove_part_name(args.file.archive).unwrap())?;
    }
    Ok(())
}
