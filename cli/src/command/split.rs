use crate::{
    cli::ArchiveFileArgs,
    command::{
        Command,
        core::{ArchiveSource, write_split_archive},
    },
    utils::PathWithCwd,
};
use anyhow::{Context, ensure};
use bytesize::ByteSize;
use clap::{ArgAction, Parser, ValueHint};
use pna::{Archive, MIN_SPLIT_PART_BYTES, prelude::PathPartExt};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SplitCommand {
    #[command(flatten)]
    archive: ArchiveFileArgs,
    #[arg(
        long,
        value_name = "BASE_PATH",
        help = "Base path used to name split archive parts. Relative paths resolve under --out-dir when given",
        value_hint = ValueHint::FilePath
    )]
    output: Option<PathBuf>,
    #[arg(long, value_name = "DIRECTORY", help = "Output directory for split archives", value_hint = ValueHint::DirPath)]
    out_dir: Option<PathBuf>,
    #[arg(long, conflicts_with = "no_overwrite", help = "Overwrite file")]
    overwrite: bool,
    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Do not overwrite files. This is the inverse option of --overwrite"
    )]
    no_overwrite: (),
    #[arg(
        long,
        value_name = "size",
        help = "Maximum size in bytes of split archive (minimum 64B)"
    )]
    pub(crate) max_size: Option<ByteSize>,
}

impl Command for SplitCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        split_archive(self)
    }
}

fn ensure_not_self_overwrite(input: &Path, out_base: &Path) -> anyhow::Result<()> {
    let first_part = out_base.with_part(1);
    match is_same_file(&first_part, input) {
        Some(true) => anyhow::bail!(
            "splitting `{}` in place would overwrite the input; specify --out-dir or --output",
            PathWithCwd::new(input)
        ),
        Some(false) => Ok(()),
        None => Err(anyhow::anyhow!(
            "failed to compare split output `{}` with input `{}`; refusing to split because the output may overwrite the input",
            PathWithCwd::new(&first_part),
            PathWithCwd::new(input)
        )),
    }
}

fn is_same_file(left: &Path, right: &Path) -> Option<bool> {
    #[cfg(not(target_os = "wasi"))]
    if let Ok(result) = same_file::is_same_file(left, right) {
        return Some(result);
    }
    match (resolved_path(left), resolved_path(right)) {
        (Some(left), Some(right)) => Some(left == right),
        _ => None,
    }
}

fn resolved_path(path: &Path) -> Option<PathBuf> {
    let mut unresolved = Vec::new();
    let mut current = path;
    for _ in 0..path.components().count() + 2 {
        if let Ok(mut resolved) = fs::canonicalize(current) {
            for part in unresolved.iter().rev() {
                resolved.push(part);
            }
            return Some(resolved);
        }
        unresolved.push(current.file_name()?);
        current = match current.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            Some(_) => Path::new("."),
            None => return None,
        };
    }
    None
}

#[hooq::hooq(anyhow)]
fn split_archive(args: SplitCommand) -> anyhow::Result<()> {
    let max_file_size = usize::try_from(args.max_size.unwrap_or_else(|| ByteSize::gb(1)).as_u64())
        .context("--max-size is too large for this platform")?;
    ensure!(
        max_file_size >= MIN_SPLIT_PART_BYTES,
        "The value for --max-size must be at least {MIN_SPLIT_PART_BYTES} bytes ({}).",
        ByteSize::b(MIN_SPLIT_PART_BYTES as u64)
    );

    let source = args.archive.source();
    let source_path = match &source {
        ArchiveSource::File(path) => Some(path.as_path()),
        ArchiveSource::Stdin => None,
    };
    let requested_output = args
        .output
        .as_deref()
        .or(source_path)
        .context(
            "archive input from standard input requires --output BASE_PATH to name the split parts",
        )?
        .to_path_buf();
    let base_out_file_name = match (&args.out_dir, &args.output) {
        (Some(out_dir), Some(_)) => out_dir.join(requested_output),
        (Some(out_dir), None) => out_dir.join(requested_output.file_name().unwrap_or_default()),
        (None, _) => requested_output.to_path_buf(),
    };
    if let Some(parent) = base_out_file_name.parent() {
        fs::create_dir_all(parent)?;
    }

    match source {
        ArchiveSource::File(archive_path) => split_file_archive(
            &archive_path,
            &base_out_file_name,
            max_file_size,
            args.overwrite,
        ),
        ArchiveSource::Stdin => split_reader_archive(
            io::stdin().lock(),
            &base_out_file_name,
            max_file_size,
            args.overwrite,
        ),
    }
    .with_context(|| {
        format!(
            "failed to create `{}`",
            PathWithCwd::new(&base_out_file_name)
        )
    })
}

fn split_reader_archive(
    reader: impl io::Read,
    output: &std::path::Path,
    max_file_size: usize,
    overwrite: bool,
) -> anyhow::Result<()> {
    let mut archive = Archive::read_header(reader)?;
    write_split_archive(output, archive.raw_entries(), max_file_size, overwrite)
}

fn split_file_archive(
    input: &std::path::Path,
    output: &std::path::Path,
    max_file_size: usize,
    overwrite: bool,
) -> anyhow::Result<()> {
    ensure_not_self_overwrite(input, output)?;
    let file = fs::File::open(input)?;
    #[cfg(not(feature = "memmap"))]
    {
        split_reader_archive(file, output, max_file_size, overwrite)
    }
    #[cfg(feature = "memmap")]
    {
        let mapped_file = crate::utils::mmap::Mmap::try_from(file)?;
        let mut archive = Archive::read_header_from_slice(&mapped_file[..])?;
        write_split_archive(
            output,
            archive.raw_entries_slice(),
            max_file_size,
            overwrite,
        )
    }
}

#[cfg(test)]
#[cfg(not(target_family = "wasm"))]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("pna_split_self_overwrite_test")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ensure_not_self_overwrite_rejects_first_part_matching_input() {
        let dir = test_dir("rejects_first_part_matching_input");
        let input = dir.join("test.part1.pna");
        fs::write(&input, b"archive").unwrap();

        let result = ensure_not_self_overwrite(&input, &dir.join("test.pna"));

        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("would overwrite the input"),
            "error should explain the self-overwrite: {err}"
        );
    }

    #[test]
    fn ensure_not_self_overwrite_allows_different_first_part() {
        let dir = test_dir("allows_different_first_part");
        let input = dir.join("test.part1.pna");
        fs::write(&input, b"archive").unwrap();

        ensure_not_self_overwrite(&input, &dir.join("other.pna")).unwrap();
    }

    #[test]
    fn ensure_not_self_overwrite_allows_output_in_new_directory() {
        let dir = test_dir("allows_output_in_new_directory");
        let input = dir.join("test.part1.pna");
        fs::write(&input, b"archive").unwrap();

        ensure_not_self_overwrite(&input, &dir.join("new").join("nested").join("test.pna"))
            .unwrap();
    }

    #[test]
    fn ensure_not_self_overwrite_rejects_hard_linked_first_part() {
        let dir = test_dir("rejects_hard_linked_first_part");
        let input = dir.join("test.part1.pna");
        fs::write(&input, b"archive").unwrap();
        fs::hard_link(&input, dir.join("alias.part1.pna")).unwrap();

        let result = ensure_not_self_overwrite(&input, &dir.join("alias.pna"));

        assert!(
            result.is_err(),
            "splitting onto a hard link of the input should be rejected"
        );
    }
}
