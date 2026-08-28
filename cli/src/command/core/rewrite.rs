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
    if let Some(patterns) = transform.patterns() {
        patterns.ensure_all_matched()?;
    }
    staged.commit()?;
    Ok(())
}

#[cfg(test)]
#[cfg(not(target_family = "wasm"))]
mod tests {
    use super::*;
    use pna::{Archive, FileEntryBuilder};
    use std::fs;

    /// Drops every entry so a committed rewrite is distinguishable from the original bytes.
    struct DropAll<'s>(GlobPatterns<'s>);

    impl EntryTransform for DropAll<'_> {
        fn transform<'d>(
            &mut self,
            entry: NormalEntry<Cow<'d, [u8]>>,
        ) -> io::Result<Option<NormalEntry<Cow<'d, [u8]>>>> {
            self.0.matches_any(entry.name());
            Ok(None)
        }

        fn patterns(&self) -> Option<&GlobPatterns<'_>> {
            Some(&self.0)
        }
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("pna_rewrite_test").join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_archive(path: &Path) -> Vec<u8> {
        let mut archive = Archive::write_header(Vec::new()).unwrap();
        archive
            .add_entry(
                FileEntryBuilder::new("present.txt".into())
                    .unwrap()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let bytes = archive.finalize().unwrap();
        fs::write(path, &bytes).unwrap();
        bytes
    }

    fn entries_beside(dir: &Path, archive: &str) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name != archive)
            .collect()
    }

    fn rewrite(archive: &Path, pattern: &str) -> anyhow::Result<()> {
        execute_archive_transform(
            archive,
            archive.to_path_buf(),
            Umask::new(0o022),
            None,
            SolidEntriesTransformStrategy::UnSolid,
            DropAll(GlobPatterns::new([pattern]).unwrap()),
        )
    }

    #[test]
    fn unmatched_pattern_refuses_the_rewrite_and_keeps_the_original() {
        let dir = test_dir("unmatched");
        let archive = dir.join("archive.pna");
        let original = write_archive(&archive);

        assert!(rewrite(&archive, "absent").is_err());
        assert_eq!(fs::read(&archive).unwrap(), original);
        assert!(entries_beside(&dir, "archive.pna").is_empty());
    }

    #[test]
    fn matched_pattern_replaces_the_original() {
        let dir = test_dir("matched");
        let archive = dir.join("archive.pna");
        let original = write_archive(&archive);

        rewrite(&archive, "present.txt").unwrap();
        assert_ne!(fs::read(&archive).unwrap(), original);
        assert!(entries_beside(&dir, "archive.pna").is_empty());
    }
}
