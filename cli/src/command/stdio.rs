use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, PasswordArgs, Verbosity},
    command::{
        ask_password, check_password,
        commons::{collect_items, entry_option, KeepOptions},
        create::create_archive_file,
        extract::{run_extract_archive_reader, OutputOption},
        Command,
    },
    utils::part_name,
};
use clap::{ArgGroup, Args, Parser, ValueHint};
use std::{
    fs,
    io::{self, stdin, stdout},
    path::PathBuf,
};

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
#[command(
  group(ArgGroup::new("bundled-flags").args(["create", "extract"])),
)]
pub(crate) struct StdioCommand {
    #[arg(short, long, help = "Create archive")]
    create: bool,
    #[arg(short = 'x', long, help = "Extract archive")]
    extract: bool,
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    recursive: bool,
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[arg(long, help = "Archiving the directories")]
    keep_dir: bool,
    #[arg(long, help = "Archiving the timestamp of the files")]
    keep_timestamp: bool,
    #[arg(long, help = "Archiving the permissions of the files")]
    keep_permission: bool,
    #[arg(long, help = "Archiving the extended attributes of the files")]
    keep_xattr: bool,
    #[arg(long, help = "Solid mode archive")]
    pub(crate) solid: bool,
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    pub(crate) exclude: Option<Vec<PathBuf>>,
    #[arg(long, help = "Output directory of extracted files", value_hint = ValueHint::DirPath)]
    pub(crate) out_dir: Option<PathBuf>,
    #[arg(short, long, help = "Input archive file path")]
    file: Option<PathBuf>,
    #[arg(help = "Files or patterns")]
    files: Vec<String>,
}

impl Command for StdioCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        run_stdio(self, verbosity)
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct FileArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    pub(crate) files: Vec<PathBuf>,
}

fn run_stdio(args: StdioCommand, verbosity: Verbosity) -> io::Result<()> {
    if args.create {
        run_create_archive(args, verbosity)
    } else if args.extract {
        run_extract_archive(args, verbosity)
    } else {
        unreachable!()
    }
}

fn run_create_archive(args: StdioCommand, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);

    let target_items = collect_items(
        &args
            .files
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
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
    if let Some(file) = args.file {
        create_archive_file(
            || fs::File::open(&file),
            cli_option,
            keep_options,
            args.solid,
            target_items,
            verbosity,
        )
    } else {
        create_archive_file(
            || Ok(stdout().lock()),
            cli_option,
            keep_options,
            args.solid,
            target_items,
            verbosity,
        )
    }
}

fn run_extract_archive(args: StdioCommand, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let out_option = OutputOption {
        overwrite: args.overwrite,
        out_dir: args.out_dir,
        keep_options: KeepOptions {
            keep_timestamp: args.keep_timestamp,
            keep_permission: args.keep_permission,
            keep_xattr: args.keep_xattr,
        },
    };
    if let Some(file) = args.file {
        run_extract_archive_reader(
            fs::File::open(&file)?,
            args.files
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>(),
            || password.as_deref(),
            |i| fs::File::open(part_name(&file, i).unwrap()),
            out_option,
            verbosity,
        )
    } else {
        run_extract_archive_reader(
            stdin().lock(),
            args.files
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>(),
            || password.as_deref(),
            |_i| Ok(stdin().lock()),
            out_option,
            verbosity,
        )
    }
}
