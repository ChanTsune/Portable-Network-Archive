use crate::{
    chunk,
    cli::{
        FileArgs, PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs,
    },
    command::{
        Command, ask_password,
        core::{
            TransformStrategyKeepSolid, TransformStrategyUnSolid, collect_split_archives,
            run_entries, run_transform_entry,
        },
    },
    ext::NormalEntryExt,
    utils::{GlobPatterns, PathPartExt, env::NamedTempFile},
};
use clap::{Parser, ValueHint};
use pna::{Chunk, NormalEntry, RawChunk};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs, io,
    str::FromStr,
};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct FflagCommand {
    #[command(subcommand)]
    command: FflagCommands,
}

impl Command for FflagCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        match self.command {
            FflagCommands::Get(cmd) => cmd.execute(ctx),
            FflagCommands::Set(cmd) => cmd.execute(ctx),
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum FflagCommands {
    #[command(about = "Get file flags of entries")]
    Get(GetFflagCommand),
    #[command(about = "Set file flags of entries")]
    Set(SetFflagCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct GetFflagCommand {
    #[command(flatten)]
    file: FileArgs,
    #[arg(short, long, help = "Show only if entry has this specific flag")]
    name: Option<String>,
    #[arg(short, long, help = "Output in restorable format")]
    dump: bool,
    #[arg(
        short = 'm',
        long = "match",
        value_name = "PATTERN",
        help = "Filter flags by regex pattern. Specify '-' for all flags"
    )]
    regex_match: Option<String>,
    #[arg(short, long, help = "Show verbose output with flag descriptions")]
    long: bool,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for GetFflagCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        archive_get_fflag(self)
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SetFflagCommand {
    #[arg(
        short = 'f',
        long = "file",
        value_name = "ARCHIVE",
        help = "Archive file path",
        value_hint = ValueHint::FilePath
    )]
    archive: std::path::PathBuf,
    #[arg(
        value_name = "FLAGS",
        help = "Comma-separated flags to set/clear (chflags-style: uchg, nouchg, nodump, dump, etc.)"
    )]
    flags: Option<FlagOperations>,
    #[arg(value_name = "FILES", help = "Entry paths to modify (supports globs)")]
    files: Vec<String>,
    #[arg(
        long,
        value_name = "FILE",
        help = "Restore flags from dump file. Use '-' for stdin",
        value_hint = ValueHint::FilePath
    )]
    restore: Option<String>,
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SetFflagCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        archive_set_fflag(self)
    }
}

/// Known file flags with their canonical names, aliases, and descriptions.
const KNOWN_FLAGS: &[(&str, &[&str], &str)] = &[
    // BSD/macOS user-level flags
    ("uchg", &["uimmutable", "uchange"], "user immutable"),
    ("uappnd", &["uappend"], "user append-only"),
    ("nodump", &[], "no dump"),
    ("hidden", &["uhidden"], "hidden file"),
    ("opaque", &[], "opaque directory"),
    ("uunlnk", &["uunlink"], "user undeletable"),
    // BSD/macOS system-level flags
    ("schg", &["simmutable", "schange"], "system immutable"),
    ("sappnd", &["sappend"], "system append-only"),
    ("archived", &["arch"], "archived"),
    ("sunlnk", &["sunlink"], "system undeletable"),
    // Linux-specific flags
    ("noatime", &[], "no atime updates"),
    ("compr", &["compress"], "compress file"),
    ("nocow", &[], "no copy-on-write"),
];

fn get_flag_description(flag: &str) -> Option<&'static str> {
    KNOWN_FLAGS
        .iter()
        .find(|(canonical, aliases, _)| *canonical == flag || aliases.contains(&flag))
        .map(|(_, _, desc)| *desc)
}

fn normalize_flag_name(flag: &str) -> String {
    // Find canonical name for aliases
    for (canonical, aliases, _) in KNOWN_FLAGS {
        if *canonical == flag {
            return flag.to_string();
        }
        if aliases.contains(&flag) {
            return (*canonical).to_string();
        }
    }
    // Unknown flag, return as-is
    flag.to_string()
}

fn is_known_flag(flag: &str) -> bool {
    KNOWN_FLAGS
        .iter()
        .any(|(canonical, aliases, _)| *canonical == flag || aliases.contains(&flag))
}

/// A single flag operation: set or clear.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum FlagOp {
    Set(String),
    Clear(String),
}

/// A collection of flag operations parsed from chflags-style input.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub(crate) struct FlagOperations(Vec<FlagOp>);

impl FromStr for FlagOperations {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ops = s
            .split(',')
            .map(|f| f.trim().to_lowercase())
            .filter(|f| !f.is_empty())
            .map(|f| parse_flag_op(&f))
            .collect::<Vec<_>>();
        Ok(Self(ops))
    }
}

fn parse_flag_op(flag: &str) -> FlagOp {
    // Special case: "dump" clears "nodump"
    if flag == "dump" {
        return FlagOp::Clear("nodump".into());
    }
    // "no" prefix clears the flag (except for "nodump" which sets the flag)
    if let Some(base) = flag.strip_prefix("no") {
        // "nodump" is a flag itself, not "no" + "dump"
        if base == "dump" {
            return FlagOp::Set(normalize_flag_name(flag));
        }
        if is_known_flag(base) || is_known_flag(flag) {
            // If base is known, it's a clear operation
            if is_known_flag(base) {
                return FlagOp::Clear(normalize_flag_name(base));
            }
        }
        // Check if the full "no*" form is itself a known flag
        if is_known_flag(flag) {
            return FlagOp::Set(normalize_flag_name(flag));
        }
        // Unknown but has "no" prefix - treat as clear
        log::warn!("Unknown flag: {}", base);
        return FlagOp::Clear(base.into());
    }
    // No "no" prefix - set the flag
    if !is_known_flag(flag) {
        log::warn!("Unknown flag: {}", flag);
    }
    FlagOp::Set(normalize_flag_name(flag))
}

impl FlagOperations {
    fn apply(&self, current_flags: &[String]) -> Vec<String> {
        let mut flags: HashSet<String> = current_flags.iter().cloned().collect();

        // Apply clears first, then sets
        for op in &self.0 {
            if let FlagOp::Clear(flag) = op {
                flags.remove(flag);
            }
        }
        for op in &self.0 {
            if let FlagOp::Set(flag) = op {
                flags.insert(flag.clone());
            }
        }

        let mut result: Vec<_> = flags.into_iter().collect();
        result.sort();
        result
    }
}

enum MatchStrategy<'s> {
    All,
    Named(&'s str),
    Regex(Regex),
}

struct FilterOption<'s> {
    dump: bool,
    long: bool,
    matcher: MatchStrategy<'s>,
}

impl<'a> FilterOption<'a> {
    fn new(
        dump: bool,
        long: bool,
        name: Option<&'a str>,
        regex_match: Option<&'a str>,
    ) -> Result<Self, regex::Error> {
        Ok(match (name, regex_match) {
            (None, None) | (None, Some("-")) => Self {
                dump,
                long,
                matcher: MatchStrategy::All,
            },
            (None, Some(re)) => Self {
                dump,
                long,
                matcher: MatchStrategy::Regex(Regex::new(re)?),
            },
            (Some(name), _) => Self {
                dump,
                long,
                matcher: MatchStrategy::Named(name),
            },
        })
    }

    fn is_match(&self, flag: &str) -> bool {
        match &self.matcher {
            MatchStrategy::All => true,
            MatchStrategy::Named(n) => *n == flag,
            MatchStrategy::Regex(re) => re.is_match(flag),
        }
    }
}

#[hooq::hooq(anyhow)]
fn archive_get_fflag(args: GetFflagCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    if args.file.files.is_empty() {
        return Ok(());
    }
    let files = &args.file.files;
    let mut globs = GlobPatterns::new(files.iter().map(|it| it.as_str()))?;
    let filter = FilterOption::new(
        args.dump,
        args.long,
        args.name.as_deref(),
        args.regex_match.as_deref(),
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let archives = collect_split_archives(&args.file.archive)?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    run_entries(
        archives,
        || password.as_deref(),
        #[hooq::skip_all]
        |entry| {
            let entry = entry?;
            let name = entry.name();
            if globs.matches_any(name) {
                let flags: Vec<_> = entry
                    .fflags()
                    .into_iter()
                    .filter(|f| filter.is_match(f))
                    .collect();

                if !flags.is_empty() || filter.dump {
                    println!("# file: {name}");
                    if filter.dump {
                        // Dump format: flags=flag1,flag2,...
                        println!("flags={}", flags.join(","));
                    } else if filter.long {
                        // Long format with descriptions
                        for flag in &flags {
                            if let Some(desc) = get_flag_description(flag) {
                                println!("{:<12} {}", flag, desc);
                            } else {
                                println!("{}", flag);
                            }
                        }
                    } else {
                        // Simple format: one flag per line
                        for flag in &flags {
                            println!("{}", flag);
                        }
                    }
                    println!();
                }
            }
            Ok(())
        },
    )?;
    globs.ensure_all_matched()?;
    Ok(())
}

#[hooq::hooq(anyhow)]
fn archive_set_fflag(args: SetFflagCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let files = &args.files;

    let mut set_strategy = if let Some("-") = args.restore.as_deref() {
        SetFflagStrategy::Restore(parse_fflag_dump(io::stdin().lock())?)
    } else if let Some(path) = args.restore.as_deref() {
        SetFflagStrategy::Restore(parse_fflag_dump(io::BufReader::new(fs::File::open(path)?))?)
    } else if files.is_empty() {
        return Ok(());
    } else {
        let globs = GlobPatterns::new(files.iter().map(|it| it.as_str()))?;
        SetFflagStrategy::Apply {
            globs,
            operations: args.flags.unwrap_or_default(),
        }
    };

    let archives = collect_split_archives(&args.archive)?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    let output_path = args.archive.remove_part();
    let mut temp_file =
        NamedTempFile::new(|| output_path.parent().unwrap_or_else(|| ".".as_ref()))?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            #[hooq::skip_all]
            |entry| Ok(Some(set_strategy.transform_entry(entry?))),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            #[hooq::skip_all]
            |entry| Ok(Some(set_strategy.transform_entry(entry?))),
            TransformStrategyKeepSolid,
        ),
    }?;

    #[cfg(feature = "memmap")]
    drop(mmaps);

    temp_file.persist(output_path)?;

    if let SetFflagStrategy::Apply { globs, .. } = set_strategy {
        globs.ensure_all_matched()?;
    }
    Ok(())
}

enum SetFflagStrategy<'s> {
    Restore(HashMap<String, Vec<String>>),
    Apply {
        globs: GlobPatterns<'s>,
        operations: FlagOperations,
    },
}

impl SetFflagStrategy<'_> {
    fn transform_entry<T>(&mut self, entry: NormalEntry<T>) -> NormalEntry<T>
    where
        T: Clone,
        RawChunk<T>: Chunk,
        RawChunk<T>: From<RawChunk>,
    {
        match self {
            Self::Restore(restore) => {
                if let Some(flags) = restore.get(entry.name().as_str()) {
                    transform_entry_flags(entry, flags)
                } else {
                    entry
                }
            }
            Self::Apply { globs, operations } => {
                if globs.matches_any(entry.name()) {
                    let current_flags = entry.fflags();
                    let new_flags = operations.apply(&current_flags);
                    transform_entry_flags(entry, &new_flags)
                } else {
                    entry
                }
            }
        }
    }
}

fn transform_entry_flags<T>(entry: NormalEntry<T>, flags: &[String]) -> NormalEntry<T>
where
    T: Clone,
    RawChunk<T>: Chunk,
    RawChunk<T>: From<RawChunk>,
{
    // Remove existing ffLg chunks
    let extra_without_fflags: Vec<_> = entry
        .extra_chunks()
        .iter()
        .filter(|c| c.ty() != chunk::ffLg)
        .cloned()
        .collect();

    // Add new ffLg chunks
    let mut extra_chunks = extra_without_fflags;
    for flag in flags {
        extra_chunks.push(chunk::fflag_chunk(flag).into());
    }

    entry.with_extra_chunks(extra_chunks)
}

fn parse_fflag_dump(reader: impl io::BufRead) -> io::Result<HashMap<String, Vec<String>>> {
    let mut result = HashMap::new();
    let mut current_file: Option<String> = None;

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        if let Some(path) = line.strip_prefix("# file: ") {
            current_file = Some(path.to_string());
        } else if let Some(file) = &current_file {
            if let Some(flags_str) = line.strip_prefix("flags=") {
                let flags: Vec<String> = flags_str
                    .split(',')
                    .map(|f| f.trim().to_string())
                    .filter(|f| !f.is_empty())
                    .collect();
                result.insert(file.clone(), flags);
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flag_op_set() {
        assert_eq!(parse_flag_op("uchg"), FlagOp::Set("uchg".into()));
        assert_eq!(parse_flag_op("schg"), FlagOp::Set("schg".into()));
        assert_eq!(parse_flag_op("hidden"), FlagOp::Set("hidden".into()));
    }

    #[test]
    fn parse_flag_op_clear() {
        assert_eq!(parse_flag_op("nouchg"), FlagOp::Clear("uchg".into()));
        assert_eq!(parse_flag_op("noschg"), FlagOp::Clear("schg".into()));
        assert_eq!(parse_flag_op("nohidden"), FlagOp::Clear("hidden".into()));
    }

    #[test]
    fn parse_flag_op_nodump_special() {
        // "nodump" is a flag itself
        assert_eq!(parse_flag_op("nodump"), FlagOp::Set("nodump".into()));
        // "dump" clears "nodump"
        assert_eq!(parse_flag_op("dump"), FlagOp::Clear("nodump".into()));
    }

    #[test]
    fn parse_flag_operations() {
        let ops: FlagOperations = "uchg,nodump,nohidden".parse().unwrap();
        assert_eq!(ops.0.len(), 3);
        assert_eq!(ops.0[0], FlagOp::Set("uchg".into()));
        assert_eq!(ops.0[1], FlagOp::Set("nodump".into()));
        assert_eq!(ops.0[2], FlagOp::Clear("hidden".into()));
    }

    #[test]
    fn apply_flag_operations() {
        let ops: FlagOperations = "uchg,nodump".parse().unwrap();
        let result = ops.apply(&[]);
        assert!(result.contains(&"uchg".to_string()));
        assert!(result.contains(&"nodump".to_string()));
    }

    #[test]
    fn apply_flag_operations_clear() {
        let ops: FlagOperations = "nouchg".parse().unwrap();
        let current = vec!["uchg".to_string(), "nodump".to_string()];
        let result = ops.apply(&current);
        assert!(!result.contains(&"uchg".to_string()));
        assert!(result.contains(&"nodump".to_string()));
    }

    #[test]
    fn apply_flag_operations_mixed() {
        let ops: FlagOperations = "schg,nouchg,nodump".parse().unwrap();
        let current = vec!["uchg".to_string()];
        let result = ops.apply(&current);
        assert!(!result.contains(&"uchg".to_string()));
        assert!(result.contains(&"schg".to_string()));
        assert!(result.contains(&"nodump".to_string()));
    }

    #[test]
    fn normalize_aliases() {
        assert_eq!(normalize_flag_name("uimmutable"), "uchg");
        assert_eq!(normalize_flag_name("simmutable"), "schg");
        assert_eq!(normalize_flag_name("uhidden"), "hidden");
        assert_eq!(normalize_flag_name("uappend"), "uappnd");
    }
}
