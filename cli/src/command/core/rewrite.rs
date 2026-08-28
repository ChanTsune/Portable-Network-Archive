use super::{
    SplitArchiveReader, StagedArchive, TransformStrategyKeepSolid, TransformStrategyUnSolid, Umask,
    collect_split_archives,
};
use crate::{cli::SolidEntriesTransformStrategy, utils::GlobPatterns};
use pna::NormalEntry;
use std::{
    borrow::Cow,
    io,
    path::{Path, PathBuf},
};

/// The per-entry step of a command that rewrites an archive.
///
/// Returning `None` drops the entry from the output.
pub(crate) trait EntryTransform {
    fn transform<'d>(
        &mut self,
        entry: NormalEntry<Cow<'d, [u8]>>,
    ) -> io::Result<Option<NormalEntry<Cow<'d, [u8]>>>>;

    /// The patterns the run was asked to match, when the command takes any.
    /// A pattern that matched nothing aborts the rewrite before the original is replaced.
    fn patterns(&self) -> Option<&GlobPatterns<'_>>;
}

/// Rewrites `archive` into `output` through `transform`, one entry at a time.
#[hooq::hooq(anyhow)]
pub(crate) fn execute_archive_transform(
    archive: &Path,
    output: PathBuf,
    umask: Umask,
    password: Option<&[u8]>,
    strategy: SolidEntriesTransformStrategy,
    mut transform: impl EntryTransform,
) -> anyhow::Result<()> {
    let mut source = SplitArchiveReader::new(collect_split_archives(archive)?)?;
    let mut staged = StagedArchive::new(output, umask)?;
    match strategy {
        SolidEntriesTransformStrategy::UnSolid => source.transform_entries(
            staged.as_file_mut(),
            password,
            #[hooq::skip_all]
            |entry| transform.transform(entry?),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => source.transform_entries(
            staged.as_file_mut(),
            password,
            #[hooq::skip_all]
            |entry| transform.transform(entry?),
            TransformStrategyKeepSolid,
        ),
    }?;
    drop(source);
    staged.commit(transform.patterns())
}
