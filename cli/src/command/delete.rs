use crate::{
    cli::{
        FileArgs, PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs,
    },
    command::{
        Command, ask_password,
        core::{
            PathFilter, TransformStrategyKeepSolid, TransformStrategyUnSolid,
            collect_split_archives, read_paths, read_paths_stdin, run_transform_entry,
        },
    },
    utils::{GlobPatterns, PathPartExt, VCS_FILES, env::NamedTempFile},
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::NormalEntry;
use std::path::PathBuf;

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(
    group(ArgGroup::new("unstable-files-from").args(["files_from"]).requires("unstable")),
    group(ArgGroup::new("unstable-files-from-stdin").args(["files_from_stdin"]).requires("unstable")),
    group(ArgGroup::new("unstable-include").args(["include"]).requires("unstable")),
    group(ArgGroup::new("unstable-delete-exclude").args(["exclude"]).requires("unstable")),
    group(ArgGroup::new("unstable-exclude-from").args(["exclude_from"]).requires("unstable")),
    group(ArgGroup::new("read-files-from").args(["files_from", "files_from_stdin"])),
    group(ArgGroup::new("unstable-exclude-vcs").args(["exclude_vcs"]).requires("unstable")),
    group(
        ArgGroup::new("from-input")
            .args(["files_from", "files_from_stdin", "exclude_from"])
            .multiple(true)
    ),
    group(ArgGroup::new("null-requires").arg("null").requires("from-input")),
)]
pub(crate) struct DeleteCommand {
    #[arg(long, help = "Output file path", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    #[arg(long, help = "Read deleting files from given path (unstable)", value_hint = ValueHint::FilePath)]
    files_from: Option<PathBuf>,
    #[arg(long, help = "Read deleting files from stdin (unstable)")]
    files_from_stdin: bool,
    #[arg(
        long,
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)"
    )]
    include: Option<Vec<String>>,
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    exclude: Option<Vec<String>>,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    exclude_from: Option<PathBuf>,
    #[arg(long, help = "Exclude vcs files (unstable)")]
    exclude_vcs: bool,
    #[arg(
        long,
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    file: FileArgs,
}

impl Command for DeleteCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        delete_file_from_archive(self)
    }
}

fn delete_file_from_archive(args: DeleteCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let mut files = args.file.files;
    if args.files_from_stdin {
        files.extend(read_paths_stdin(args.null)?);
    } else if let Some(path) = args.files_from {
        files.extend(read_paths(path, args.null)?);
    }
    let mut globs = GlobPatterns::new(files.iter().map(|it| it.as_str()))?;

    let mut exclude = args.exclude.unwrap_or_default();
    if let Some(p) = args.exclude_from {
        exclude.extend(read_paths(p, args.null)?);
    }
    let vcs_patterns = args
        .exclude_vcs
        .then(|| VCS_FILES.iter().copied())
        .into_iter()
        .flatten();
    let filter = PathFilter::new(
        args.include.iter().flatten(),
        exclude.iter().map(|s| s.as_str()).chain(vcs_patterns),
    );

    let archives = collect_split_archives(&args.file.archive)?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<std::io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    let output_path = args
        .output
        .unwrap_or_else(|| args.file.archive.remove_part());
    let mut temp_file =
        NamedTempFile::new(|| output_path.parent().unwrap_or_else(|| ".".as_ref()))?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            |entry| Ok(filter_entry(&mut globs, &filter, entry?)),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            |entry| Ok(filter_entry(&mut globs, &filter, entry?)),
            TransformStrategyKeepSolid,
        ),
    }?;

    #[cfg(feature = "memmap")]
    drop(mmaps);

    temp_file.persist(output_path)?;

    globs.ensure_all_matched()?;
    Ok(())
}

#[inline]
fn filter_entry<T>(
    globs: &mut GlobPatterns<'_>,
    filter: &PathFilter,
    entry: NormalEntry<T>,
) -> Option<NormalEntry<T>> {
    let entry_path = entry.header().path();
    if globs.matches_any(entry_path) && !filter.excluded(entry_path) {
        return None;
    }
    Some(entry)
}
