mod old_style;
pub mod value;

#[doc(hidden)]
pub use old_style::expand_stdio_old_style_args;

use crate::{
    command::{
        append::AppendCommand, bugreport::BugReportCommand, complete::CompleteCommand,
        concat::ConcatCommand, core::Umask, create::CreateCommand, delete::DeleteCommand,
        experimental::ExperimentalCommand, extract::ExtractCommand, list::ListCommand,
        sort::SortCommand, split::SplitCommand, strip::StripCommand, xattr::XattrCommand,
    },
    utils::{fs::current_umask, process::is_running_as_root},
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

#[derive(Parser, Clone, Eq, PartialEq, Debug, Default)]
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

/// Runtime context for command execution.
///
/// This struct contains [`GlobalArgs`] and adds computed values that must be
/// initialized early in the process lifecycle (before spawning threads).
///
/// # Thread Safety
///
/// `GlobalContext` should be created before any parallel processing.
/// The umask is captured at construction time to minimize exposure to the
/// inherent race window in umask reading.
#[derive(Debug)]
pub(crate) struct GlobalContext {
    args: GlobalArgs,
    umask: Umask,
    is_root: bool,
}

impl GlobalContext {
    /// Creates a new execution context, capturing runtime values.
    ///
    /// This MUST be called before spawning any threads that may create files,
    /// as umask reading involves a brief race window.
    pub(crate) fn new(args: GlobalArgs) -> Self {
        Self {
            umask: Umask::new(current_umask()),
            is_root: is_running_as_root(),
            args,
        }
    }

    /// Returns the cached umask value.
    #[inline]
    pub(crate) fn umask(&self) -> Umask {
        self.umask
    }

    /// Returns whether the process is running as root/Administrator.
    #[inline]
    pub(crate) fn is_root(&self) -> bool {
        self.is_root
    }

    /// Returns the color choice setting.
    #[inline]
    pub(crate) fn color(&self) -> ColorChoice {
        self.args.color()
    }

    /// Returns whether unstable features are enabled.
    #[inline]
    pub(crate) fn unstable(&self) -> bool {
        self.args.unstable
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

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[command(group(ArgGroup::new("verbosity").args(["quiet", "verbose", "log_level"])))]
pub(crate) struct VerbosityArgs {
    #[arg(
        long,
        global = true,
        help = "Make some output more quiet (alias for --log-level off)"
    )]
    quiet: bool,
    #[arg(
        long,
        global = true,
        help = "Make some output more verbose (alias for --log-level debug)"
    )]
    verbose: bool,
    #[arg(
        long,
        global = true,
        value_name = "LEVEL",
        default_value = "warn",
        help = "Set the log level"
    )]
    log_level: LogLevel,
}

impl VerbosityArgs {
    #[inline]
    pub(crate) const fn log_level_filter(&self) -> LevelFilter {
        match (self.quiet, self.verbose) {
            (true, _) => LevelFilter::Off,
            (_, true) => LevelFilter::Debug,
            (_, _) => self.log_level.as_level_filter(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_captures_umask() {
        let args = GlobalArgs::default();
        let ctx = GlobalContext::new(args);
        assert!(ctx.umask().apply(0o777) <= 0o777);
    }

    #[test]
    fn context_delegates_to_global_args() {
        let args = GlobalArgs {
            unstable: true,
            ..Default::default()
        };
        let ctx = GlobalContext::new(args);
        assert!(ctx.unstable());
    }

    #[test]
    fn context_delegates_color_to_global_args() {
        let args = GlobalArgs {
            color: ColorArgs {
                color: ColorChoice::Never,
            },
            ..Default::default()
        };
        let ctx = GlobalContext::new(args);
        assert_eq!(ctx.color(), ColorChoice::Never);
    }

    #[test]
    fn is_root_returns_consistent_result() {
        let args = GlobalArgs::default();
        let ctx1 = GlobalContext::new(args.clone());
        let ctx2 = GlobalContext::new(args);
        assert_eq!(ctx1.is_root(), ctx2.is_root());
    }

    #[test]
    fn quiet_and_log_level_conflict() {
        let result = Cli::try_parse_from([
            "pna",
            "--quiet",
            "--log-level",
            "info",
            "list",
            "-f",
            "a.pna",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn verbose_and_log_level_conflict() {
        let result = Cli::try_parse_from([
            "pna",
            "--verbose",
            "--log-level",
            "info",
            "list",
            "-f",
            "a.pna",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn quiet_and_verbose_conflict() {
        let result = Cli::try_parse_from(["pna", "--quiet", "--verbose", "list", "-f", "a.pna"]);
        assert!(result.is_err());
    }

    #[test]
    fn quiet_alone_accepted() {
        let cli = Cli::try_parse_from(["pna", "--quiet", "list", "-f", "a.pna"]).unwrap();
        assert_eq!(cli.global.verbosity.log_level_filter(), LevelFilter::Off);
    }

    #[test]
    fn verbose_alone_accepted() {
        let cli = Cli::try_parse_from(["pna", "--verbose", "list", "-f", "a.pna"]).unwrap();
        assert_eq!(cli.global.verbosity.log_level_filter(), LevelFilter::Debug);
    }

    #[test]
    fn log_level_alone_accepted() {
        let cli =
            Cli::try_parse_from(["pna", "--log-level", "debug", "list", "-f", "a.pna"]).unwrap();
        assert_eq!(cli.global.verbosity.log_level_filter(), LevelFilter::Debug);
    }
}
