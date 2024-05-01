use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, PasswordArgs, Verbosity},
    command::{
        ask_password, check_password,
        commons::{collect_items, entry_option, KeepOptions},
        create::create_archive_file,
        stdio::FileArgs,
        Command,
    },
};
use clap::{ArgGroup, Parser, ValueHint};
use std::{
    io::{self, stdout},
    path::PathBuf,
};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(group(ArgGroup::new("unstable-stdio-create-exclude").args(["exclude"]).requires("unstable")))]
pub(crate) struct CreateCommand {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Archiving the directories")]
    pub(crate) keep_dir: bool,
    #[arg(long, help = "Archiving the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Archiving the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Archiving the extended attributes of the files")]
    pub(crate) keep_xattr: bool,
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
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);

    let target_items = collect_items(
        &args.file.files,
        args.recursive,
        args.keep_dir,
        &args.exclude,
    )?;
    let cli_option = entry_option(args.compression, args.cipher, password);
    let keep_options = KeepOptions {
        keep_timestamp: args.keep_timestamp,
        keep_permission: args.keep_permission,
        keep_xattr: args.keep_xattr,
    };
    create_archive_file(
        || Ok(stdout().lock()),
        cli_option,
        keep_options,
        args.solid,
        target_items,
        verbosity,
    )
}
