pub mod value;

use crate::command::{
    append::AppendCommand, bugreport::BugReportCommand, complete::CompleteCommand,
    concat::ConcatCommand, create::CreateCommand, delete::DeleteCommand,
    experimental::ExperimentalCommand, extract::ExtractCommand, list::ListCommand,
    sort::SortCommand, split::SplitCommand, strip::StripCommand, xattr::XattrCommand,
};
use clap::{ArgGroup, Parser, Subcommand, ValueEnum, ValueHint};
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
    pub(crate) global: GlobalArgs,
}

#[derive(Parser, Clone, Eq, PartialEq, Debug)]
pub(crate) struct GlobalArgs {
    #[command(flatten)]
    pub(crate) verbosity: VerbosityArgs,
    #[command(flatten)]
    pub(crate) color: ColorArgs,
    #[arg(
        long,
        global = true,
        help = "Enable experimental options. Required for flags marked as unstable; behavior may change or be removed."
    )]
    pub(crate) unstable: bool,
}

impl GlobalArgs {
    #[inline]
    pub(crate) fn color(&self) -> ColorChoice {
        self.color.color
    }
}

impl Cli {
    pub fn init_logger(&self) -> io::Result<()> {
        let level = self.global.verbosity.log_level_filter();
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

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub(crate) struct ColorArgs {
    #[arg(
        long,
        global = true,
        value_name = "WHEN",
        default_value = "auto",
        help = "Control color output"
    )]
    pub(crate) color: ColorChoice,
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
    #[command(about = "Delete entry from archive")]
    Delete(DeleteCommand),
    #[command(about = "Split archive")]
    Split(SplitCommand),
    #[command(about = "Concat archives")]
    Concat(ConcatCommand),
    #[command(about = "Strip entries metadata")]
    Strip(StripCommand),
    #[command(about = "Sort entries in archive")]
    Sort(SortCommand),
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
    #[arg(short, long, help = "Archive file path", value_hint = ValueHint::FilePath)]
    file: Option<PathBuf>,
    #[arg(help = "Archive file path (deprecated, use --file)", value_hint = ValueHint::FilePath)]
    archive: Option<PathBuf>,
    #[arg(help = "Files or directories to process", value_hint = ValueHint::FilePath)]
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
        visible_alias = "passphrase",
        help = "Password of archive. If password is not given it's asked from the tty"
    )]
    pub(crate) password: Option<Option<String>>,
    #[arg(long, value_name = "FILE", help = "Read password from specified file")]
    pub(crate) password_file: Option<PathBuf>,
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(group(ArgGroup::new("transform_strategy").args(["unsolid", "keep_solid"])))]
pub(crate) struct SolidEntriesTransformStrategyArgs {
    #[arg(long, help = "Convert solid entries to regular entries")]
    unsolid: bool,
    #[arg(long, help = "Preserve solid entries without conversion")]
    keep_solid: bool,
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
        help = "Use deflate for compression [possible level: 1-9, min, max]"
    )]
    pub(crate) deflate: Option<Option<DeflateLevel>>,
    #[arg(
        long,
        value_name = "level",
        help = "Use zstd for compression [possible level: 1-21, min, max]"
    )]
    pub(crate) zstd: Option<Option<ZstdLevel>>,
    #[arg(
        long,
        value_name = "level",
        help = "Use xz for compression [possible level: 0-9, min, max]"
    )]
    pub(crate) xz: Option<Option<XzLevel>>,
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
    #[arg(long, value_name = "PARAMS", help = "Use argon2 for password hashing")]
    argon2: Option<Option<Argon2idParams>>,
    #[arg(long, value_name = "PARAMS", help = "Use pbkdf2 for password hashing")]
    pbkdf2: Option<Option<Pbkdf2Sha256Params>>,
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
