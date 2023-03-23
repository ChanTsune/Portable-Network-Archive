use clap::ValueEnum;
use clap::{value_parser, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version,
    about,
    author,
    arg_required_else_help = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) commands: Commands,
    #[command(flatten)]
    pub(crate) verbosity: VerbosityArgs,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct VerbosityArgs {
    #[arg(long, help = "Make some output more quiet")]
    quiet: bool,
    #[arg(long, help = "Make some output more verbose")]
    verbose: bool,
}

impl VerbosityArgs {
    pub(crate) fn verbosity(&self) -> Verbosity {
        match (self.quiet, self.verbose) {
            (true, false) => Verbosity::Quite,
            (false, true) => Verbosity::Verbose,
            (_, _) => Verbosity::Normal,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Verbosity {
    Quite,
    Normal,
    Verbose,
}

#[derive(Subcommand, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Commands {
    #[command(visible_alias = "c", about = "Create archive")]
    Create(CreateArgs),
    #[command(visible_alias = "a", about = "Append files to archive")]
    Append(AppendArgs),
    #[command(visible_alias = "x", about = "Extract files from archive")]
    Extract(ExtractArgs),
    #[command(visible_alias = "l", about = "List files in archive")]
    List(ListArgs),
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct CreateArgs {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Archiving the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Archiving the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Split archive by total entry size")]
    pub(crate) split: Option<Option<usize>>,
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct AppendArgs {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ExtractArgs {
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Output directory of extracted files")]
    pub(crate) out_dir: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[arg(long, help = "Restore the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[command(flatten)]
    pub(crate) file: FileArgs,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ListArgs {
    #[arg(short, long, help = "Display extended file metadata as a table")]
    pub(crate) long: bool,
    #[arg(short, long, help = "Add a header row to each column")]
    pub(crate) header: bool,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct FileArgs {
    #[arg()]
    pub(crate) archive: PathBuf,
    #[arg()]
    pub(crate) files: Vec<PathBuf>,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct PasswordArgs {
    #[arg(
        long,
        help = "Password of archive. If password is not given it's asked from the tty"
    )]
    pub(crate) password: Option<Option<String>>,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct CompressionAlgorithmArgs {
    #[arg(long, help = "No compression")]
    pub(crate) store: bool,
    #[arg(
        long,
        value_name = "level",
        value_parser = value_parser!(u8).range(1..=9),
        help = "Use deflate for compression [possible level: 1-9]"
    )]
    pub(crate) deflate: Option<Option<u8>>,
    #[arg(
        long,
        value_name = "level",
        value_parser = value_parser!(u8).range(1..=21),
        help = "Use zstd for compression [possible level: 1-21]"
    )]
    pub(crate) zstd: Option<Option<u8>>,
    #[arg(
        long,
        value_name = "level",
        value_parser = value_parser!(u8).range(0..=9),
        help = "Use xz for compression [possible level: 0-9]"
    )]
    pub(crate) xz: Option<Option<u8>>,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct CipherAlgorithmArgs {
    #[arg(long, value_name = "cipher mode", help = "Use aes for encryption")]
    pub(crate) aes: Option<Option<CipherMode>>,
    #[arg(long, value_name = "cipher mode", help = "use camellia for encryption")]
    pub(crate) camellia: Option<Option<CipherMode>>,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, ValueEnum)]
pub(crate) enum CipherMode {
    Cbc,
    Ctr,
}

impl Default for CipherMode {
    fn default() -> Self {
        Self::Ctr
    }
}
