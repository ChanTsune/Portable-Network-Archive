pub(crate) mod value;

use crate::command::{
    append::AppendCommand, bugreport::BugReportCommand, complete::CompleteCommand,
    concat::ConcatCommand, create::CreateCommand, experimental::ExperimentalCommand,
    extract::ExtractCommand, list::ListCommand, split::SplitCommand, strip::StripCommand,
    xattr::XattrCommand,
};
use clap::{value_parser, ArgGroup, Parser, Subcommand, ValueEnum, ValueHint};
use log::{Level, LevelFilter};
use pna::HashAlgorithm;
use std::{io, path::PathBuf, str::FromStr};
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
    #[arg(long, global = true, help = "Declare to use unstable features")]
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
    pub(crate) fn log_level_filter(&self) -> LevelFilter {
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
    #[command(about = "Unstable experimental commands")]
    Experimental(ExperimentalCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct FileArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    pub(crate) archive: PathBuf,
    #[arg(value_hint = ValueHint::FilePath)]
    pub(crate) files: Vec<String>,
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
#[command(group(ArgGroup::new("transform_strategy").args(["password", "password_file"])))]
pub(crate) struct SolidEntriesTransformStrategyArgs {
    #[arg(long, help = "Unsolid input solid entries.")]
    pub(crate) unsolid: bool,
    #[arg(long, help = "Keep input solid entries.")]
    pub(crate) keep_solid: bool,
}

impl SolidEntriesTransformStrategyArgs {
    #[inline]
    pub(crate) fn strategy(&self) -> SolidEntriesTransformStrategy {
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
    pub(crate) fn algorithm(&self) -> pna::Encryption {
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Argon2idParams {
    time: Option<u32>,
    memory: Option<u32>,
    parallelism: Option<u32>,
}

impl FromStr for Argon2idParams {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut time = None;
        let mut memory = None;
        let mut parallelism = None;
        for param in s.split(',') {
            let kv = param.split_once('=');
            if let Some(("t", n)) = kv {
                time = Some(
                    n.parse()
                        .map_err(|it: std::num::ParseIntError| it.to_string())?,
                )
            } else if let Some(("m", n)) = kv {
                memory = Some(
                    n.parse()
                        .map_err(|it: std::num::ParseIntError| it.to_string())?,
                )
            } else if let Some(("p", n)) = kv {
                parallelism = Some(
                    n.parse()
                        .map_err(|it: std::num::ParseIntError| it.to_string())?,
                )
            } else {
                return Err(format!("Unknown parameter `{param}`"));
            }
        }
        Ok(Self {
            time,
            memory,
            parallelism,
        })
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Pbkdf2Sha256Params {
    rounds: Option<u32>,
}

impl FromStr for Pbkdf2Sha256Params {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut rounds = None;
        for param in s.split(',') {
            let kv = param.split_once('=');
            if let Some(("r", n)) = kv {
                rounds = Some(
                    n.parse()
                        .map_err(|it: std::num::ParseIntError| it.to_string())?,
                )
            } else {
                return Err(format!("Unknown parameter `{param}`"));
            }
        }
        Ok(Self { rounds })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_argon2id_params() {
        assert_eq!(
            Argon2idParams::from_str("t=1,m=2,p=3"),
            Ok(Argon2idParams {
                time: Some(1),
                memory: Some(2),
                parallelism: Some(3),
            })
        );
    }

    #[test]
    fn parse_pbkdf2_sha256_params() {
        assert_eq!(
            Pbkdf2Sha256Params::from_str("r=1"),
            Ok(Pbkdf2Sha256Params { rounds: Some(1) })
        );
    }
}
