use bytesize::ByteSize;
use clap::ValueEnum;
use clap::{value_parser, ArgGroup, Parser, Subcommand};
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
    #[arg(long, global = true, help = "Declare to use unstable features")]
    pub(crate) unstable: bool,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("verbosity").args(["quiet", "verbose"])))]
pub(crate) struct VerbosityArgs {
    #[arg(long, global = true, help = "Make some output more quiet")]
    quiet: bool,
    #[arg(long, global = true, help = "Make some output more verbose")]
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
#[command(group(ArgGroup::new("unstable-split").args(["split"]).requires("unstable")))]
pub(crate) struct CreateArgs {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Archiving the directories")]
    pub(crate) keep_dir: bool,
    #[arg(long, help = "Archiving the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Archiving the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Split archive by total entry size")]
    pub(crate) split: Option<Option<ByteSize>>,
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
    #[arg(long, help = "Display solid mode archive entries")]
    pub(crate) solid: bool,
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
#[command(group(ArgGroup::new("compression_method").args(["store", "deflate", "zstd", "xz"])))]
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

impl CompressionAlgorithmArgs {
    pub(crate) fn algorithm(&self) -> (libpna::Compression, Option<libpna::CompressionLevel>) {
        if self.store {
            (libpna::Compression::No, None)
        } else if let Some(level) = self.xz {
            (
                libpna::Compression::XZ,
                level.map(libpna::CompressionLevel::from),
            )
        } else if let Some(level) = self.zstd {
            (
                libpna::Compression::ZStandard,
                level.map(libpna::CompressionLevel::from),
            )
        } else if let Some(level) = self.deflate {
            (
                libpna::Compression::Deflate,
                level.map(libpna::CompressionLevel::from),
            )
        } else {
            (libpna::Compression::ZStandard, None)
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("cipher_algorithm").args(["aes", "camellia"])))]
pub(crate) struct CipherAlgorithmArgs {
    #[arg(long, value_name = "cipher mode", help = "Use aes for encryption")]
    pub(crate) aes: Option<Option<CipherMode>>,
    #[arg(long, value_name = "cipher mode", help = "use camellia for encryption")]
    pub(crate) camellia: Option<Option<CipherMode>>,
}

impl CipherAlgorithmArgs {
    pub(crate) fn algorithm(&self) -> libpna::Encryption {
        if self.aes.is_some() {
            libpna::Encryption::Aes
        } else if self.camellia.is_some() {
            libpna::Encryption::Camellia
        } else {
            libpna::Encryption::Aes
        }
    }

    pub(crate) fn mode(&self) -> libpna::CipherMode {
        match match (self.aes, self.camellia) {
            (Some(mode), _) | (_, Some(mode)) => mode.unwrap_or_default(),
            (None, None) => CipherMode::default(),
        } {
            CipherMode::Cbc => libpna::CipherMode::CBC,
            CipherMode::Ctr => libpna::CipherMode::CTR,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn store_archive() {
        let args = CreateArgs::parse_from(["create", "c.pna"]);
        assert!(!args.compression.store);

        let args = CreateArgs::parse_from(["create", "c.pna", "--store"]);
        assert!(args.compression.store);
    }

    #[test]
    fn deflate_level() {
        let args = CreateArgs::parse_from(["create", "c.pna"]);
        assert_eq!(args.compression.deflate, None);

        let args = CreateArgs::parse_from(["create", "c.pna", "--deflate"]);
        assert_eq!(args.compression.deflate, Some(None));

        let args = CreateArgs::parse_from(["create", "c.pna", "--deflate", "5"]);
        assert_eq!(args.compression.deflate, Some(Some(5u8)));
    }

    #[test]
    fn zstd_level() {
        let args = CreateArgs::parse_from(["create", "c.pna"]);
        assert_eq!(args.compression.zstd, None);

        let args = CreateArgs::parse_from(["create", "c.pna", "--zstd"]);
        assert_eq!(args.compression.zstd, Some(None));

        let args = CreateArgs::parse_from(["create", "c.pna", "--zstd", "5"]);
        assert_eq!(args.compression.zstd, Some(Some(5u8)));
    }

    #[test]
    fn lzma_level() {
        let args = CreateArgs::parse_from(["create", "c.pna"]);
        assert_eq!(args.compression.xz, None);

        let args = CreateArgs::parse_from(["create", "c.pna", "--xz"]);
        assert_eq!(args.compression.xz, Some(None));

        let args = CreateArgs::parse_from(["create", "c.pna", "--xz", "5"]);
        assert_eq!(args.compression.xz, Some(Some(5u8)));
    }

    #[test]
    fn human_readable_byte_size() {
        let args = CreateArgs::parse_from(["create", "c.pna", "--split", "10KiB"]);
        assert_eq!(args.split, Some(Some(ByteSize::kib(10))))
    }
}
