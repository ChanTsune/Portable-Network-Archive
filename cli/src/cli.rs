pub mod value;

use crate::command::{
    append::AppendCommand, bugreport::BugReportCommand, complete::CompleteCommand,
    concat::ConcatCommand, create::CreateCommand, experimental::ExperimentalCommand,
    extract::ExtractCommand, list::ListCommand, split::SplitCommand, strip::StripCommand,
    xattr::XattrCommand,
};
use clap::{ArgGroup, Parser, Subcommand, ValueEnum, ValueHint, value_parser};
use log::{Level, LevelFilter};
use pna::HashAlgorithm;
use std::{io, path::PathBuf};
pub(crate) use value::*;

#[derive(Parser, Clone, Debug)]
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
    #[arg(
        long,
        global = true,
        help = "Enable experimental options. Required for flags marked as unstable; behavior may change or be removed."
    )]
    pub(crate) unstable: bool,
}

impl Cli {
    pub fn init_logger(&self) -> io::Result<()> {
        let level = self.verbosity.log_level_filter();
        let base = fern::Dispatch::new();
        let stderr = fern::Dispatch::new()
            .level(level)
            .format(|out, msg, rec| match rec.level() {
                Level::Error => out.finish(format_args!("error: {msg}")),
                Level::Warn => out.finish(format_args!("warning: {msg}")),
                Level::Info | Level::Debug | Level::Trace => out.finish(*msg),
            })
            .chain(io::stderr());
        base.chain(stderr).apply().map_err(io::Error::other)
    }
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
    #[inline]
    #[allow(dead_code)]
    pub(crate) const fn log_level_filter(&self) -> LevelFilter {
        match (self.quiet, self.verbose) {
            (true, false) => LevelFilter::Off,
            (false, true) => LevelFilter::Debug,
            (_, _) => LevelFilter::Info,
        }
    }
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum Commands {
    #[command(visible_alias = "c", about = "Create archive")]
    Create(CreateCommand),
    #[command(visible_alias = "a", about = "Append files to archive")]
    Append(AppendCommand),
    #[command(visible_alias = "x", about = "Extract files from archive")]
    Extract(ExtractCommand),
    #[command(visible_aliases = &["l", "ls"], about = "List files in archive")]
    List(ListCommand),
    #[command(about = "Split archive")]
    Split(SplitCommand),
    #[command(about = "Concat archives")]
    Concat(ConcatCommand),
    #[command(about = "Strip entries metadata")]
    Strip(StripCommand),
    #[command(about = "Manipulate extended attributes")]
    Xattr(XattrCommand),
    #[command(about = "Generate shell auto complete")]
    Complete(CompleteCommand),
    #[command(about = "Generate bug report template")]
    BugReport(BugReportCommand),
    #[command(
        about = "Unstable experimental commands; behavior and interface may change or be removed"
    )]
    Experimental(ExperimentalCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct FileArgs {
    #[arg(short = 'f', long = "file", value_hint = ValueHint::FilePath)]
    pub(crate) archive: PathBuf,
    #[arg(value_hint = ValueHint::FilePath)]
    pub(crate) files: Vec<String>,
}

// Archive related args for compatibility with optional and positional arguments.
// This is a temporary measure while the compatibility feature is available,
// and will be removed once the compatibility feature is no longer available.
//
// NOTE: Do NOT use doc comments (///) here as they become `long_about` in clap
// and propagate to commands that flatten this struct, causing incorrect documentation.
#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    group(
        ArgGroup::new("archive-args")
            .args(["file", "archive"])
            .multiple(true)
            .required(true)
    )
)]
pub(crate) struct FileArgsCompat {
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    file: Option<PathBuf>,
    #[arg(value_hint = ValueHint::FilePath)]
    archive: Option<PathBuf>,
    #[arg(value_hint = ValueHint::FilePath)]
    files: Vec<String>,
}

impl FileArgsCompat {
    #[inline]
    pub(crate) fn archive(&self) -> PathBuf {
        if let Some(file) = &self.file {
            file.clone()
        } else if let Some(archive) = &self.archive {
            log::warn!("positional `archive` is deprecated, use `--file` instead");
            archive.clone()
        } else {
            unreachable!()
        }
    }

    #[inline]
    pub(crate) fn files(&self) -> Vec<String> {
        if self.file.is_none() {
            self.files.clone()
        } else {
            let mut files = self.files.clone();
            if let Some(archive) = &self.archive {
                files.insert(0, archive.to_string_lossy().to_string());
            }
            files
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("password_provider").args(["password", "password_file"])))]
pub(crate) struct PasswordArgs {
    #[arg(
        long,
        help = "Password of archive. If password is not given it's asked from the tty"
    )]
    pub(crate) password: Option<Option<String>>,
    #[arg(long, help = "Read password from specified file")]
    pub(crate) password_file: Option<PathBuf>,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("transform_strategy").args(["unsolid", "keep_solid"])))]
pub(crate) struct SolidEntriesTransformStrategyArgs {
    #[arg(long, help = "Unsolid input solid entries.")]
    pub(crate) unsolid: bool,
    #[arg(long, help = "Keep input solid entries.")]
    pub(crate) keep_solid: bool,
}

impl SolidEntriesTransformStrategyArgs {
    #[inline]
    pub(crate) const fn strategy(&self) -> SolidEntriesTransformStrategy {
        if self.unsolid {
            SolidEntriesTransformStrategy::UnSolid
        } else {
            SolidEntriesTransformStrategy::KeepSolid
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum SolidEntriesTransformStrategy {
    UnSolid,
    KeepSolid,
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
    pub(crate) fn algorithm(&self) -> (pna::Compression, Option<pna::CompressionLevel>) {
        if self.store {
            (pna::Compression::No, None)
        } else if let Some(level) = self.xz {
            (pna::Compression::XZ, level.map(Into::into))
        } else if let Some(level) = self.zstd {
            (pna::Compression::ZStandard, level.map(Into::into))
        } else if let Some(level) = self.deflate {
            (pna::Compression::Deflate, level.map(Into::into))
        } else {
            (pna::Compression::ZStandard, None)
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("cipher_algorithm").args(["aes", "camellia"])))]
pub(crate) struct CipherAlgorithmArgs {
    #[arg(long, value_name = "cipher mode", help = "Use aes for encryption")]
    pub(crate) aes: Option<Option<CipherMode>>,
    #[arg(long, value_name = "cipher mode", help = "Use camellia for encryption")]
    pub(crate) camellia: Option<Option<CipherMode>>,
}

impl CipherAlgorithmArgs {
    pub(crate) const fn algorithm(&self) -> pna::Encryption {
        if self.aes.is_some() {
            pna::Encryption::Aes
        } else if self.camellia.is_some() {
            pna::Encryption::Camellia
        } else {
            pna::Encryption::Aes
        }
    }

    pub(crate) fn mode(&self) -> pna::CipherMode {
        match match (self.aes, self.camellia) {
            (Some(mode), _) | (_, Some(mode)) => mode.unwrap_or_default(),
            (None, None) => CipherMode::default(),
        } {
            CipherMode::Cbc => pna::CipherMode::CBC,
            CipherMode::Ctr => pna::CipherMode::CTR,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, ValueEnum)]
pub(crate) enum CipherMode {
    Cbc,
    #[default]
    Ctr,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("hash_algorithm").args(["argon2", "pbkdf2"])))]
pub(crate) struct HashAlgorithmArgs {
    #[arg(long, help = "Use argon2 for password hashing")]
    pub(crate) argon2: Option<Option<Argon2idParams>>,
    #[arg(long, help = "Use pbkdf2 for password hashing")]
    pub(crate) pbkdf2: Option<Option<Pbkdf2Sha256Params>>,
}

impl HashAlgorithmArgs {
    pub(crate) fn algorithm(&self) -> HashAlgorithm {
        if let Some(Some(params)) = &self.pbkdf2 {
            HashAlgorithm::pbkdf2_sha256_with(params.rounds)
        } else if self.pbkdf2.as_ref().is_some_and(|it| it.is_none()) {
            HashAlgorithm::pbkdf2_sha256()
        } else if let Some(Some(params)) = &self.argon2 {
            HashAlgorithm::argon2id_with(params.time, params.memory, params.parallelism)
        } else {
            HashAlgorithm::argon2id()
        }
    }
}
