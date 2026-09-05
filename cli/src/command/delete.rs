use crate::{
    cli::{ArchiveFileArgs, FileOperands, PasswordArgs, SolidEntriesTransformStrategyArgs},
    command::{
        Command, ask_password,
        core::{
            PathFilter, Umask, read_paths, read_paths_stdin, resolve_rewrite_output,
            rewrite::{EntryTransform, execute_archive_transform},
        },
    },
    utils::{GlobPatterns, VCS_FILES},
};
use clap::{ArgAction, ArgGroup, Parser, ValueHint};
use pna::NormalEntry;
use std::{borrow::Cow, io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(
    group(ArgGroup::new("read-files-from").args(["files_from", "files_from_stdin"])),
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
    #[arg(
        long,
        conflicts_with = "no_overwrite",
        requires = "output",
        help = "Overwrite output file"
    )]
    overwrite: bool,
    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Do not overwrite output file. This is the inverse option of --overwrite",
        requires = "output"
    )]
    no_overwrite: (),
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Read deleting files from given path",
        value_hint = ValueHint::FilePath
    )]
    files_from: Option<PathBuf>,
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Read deleting files from stdin"
    )]
    files_from_stdin: bool,
    #[arg(
        long,
        value_name = "PATTERN",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions"
    )]
    include: Vec<String>,
    #[arg(
        long,
        value_name = "PATTERN",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Exclude path glob",
        value_hint = ValueHint::AnyPath
    )]
    exclude: Vec<String>,
    #[arg(
        long,
        value_name = "FILE",
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Read exclude files from given path",
        value_hint = ValueHint::FilePath
    )]
    exclude_from: Option<PathBuf>,
    #[arg(
        long,
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`)"
    )]
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
    archive: ArchiveFileArgs,
    #[command(flatten)]
    files: FileOperands,
}

impl Command for DeleteCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        delete_file_from_archive(self, ctx.umask())
    }
}

#[hooq::hooq(anyhow)]
fn delete_file_from_archive(args: DeleteCommand, umask: Umask) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let mut files = args.files.files;
    if args.files_from_stdin {
        files.extend(read_paths_stdin(args.null)?);
    } else if let Some(path) = args.files_from {
        files.extend(read_paths(path, args.null)?);
    }
    let globs = GlobPatterns::new(files.iter().map(|it| it.as_str()))?;

    let mut exclude = args.exclude;
    if let Some(p) = args.exclude_from {
        exclude.extend(read_paths(p, args.null)?);
    }
    let vcs_patterns = args
        .exclude_vcs
        .then(|| VCS_FILES.iter().copied())
        .into_iter()
        .flatten();
    let filter = PathFilter::new(
        args.include.iter().map(|s| s.as_str()),
        exclude.iter().map(|s| s.as_str()).chain(vcs_patterns),
    );

    let destination = resolve_rewrite_output(&args.archive.file, args.output, args.overwrite)?;
    execute_archive_transform(
        &args.archive.file,
        destination,
        umask,
        password.as_deref(),
        args.transform_strategy.strategy(),
        DeleteTransform { globs, filter },
    )
}

struct DeleteTransform<'g, 'f> {
    globs: GlobPatterns<'g>,
    filter: PathFilter<'f>,
}

impl EntryTransform for DeleteTransform<'_, '_> {
    fn transform<'d>(
        &mut self,
        entry: NormalEntry<Cow<'d, [u8]>>,
    ) -> io::Result<Option<NormalEntry<Cow<'d, [u8]>>>> {
        Ok(filter_entry(&mut self.globs, &self.filter, entry))
    }

    fn patterns(&self) -> Option<&GlobPatterns<'_>> {
        Some(&self.globs)
    }
}

#[inline]
fn filter_entry<T>(
    globs: &mut GlobPatterns<'_>,
    filter: &PathFilter,
    entry: NormalEntry<T>,
) -> Option<NormalEntry<T>> {
    let entry_path = entry.name();
    if globs.matches_any(entry_path) && !filter.excluded(entry_path) {
        return None;
    }
    Some(entry)
}
