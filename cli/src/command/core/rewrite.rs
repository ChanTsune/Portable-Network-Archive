use super::{
    ArchiveSource, TransformStrategyKeepSolid, TransformStrategyUnSolid, Umask,
    archive_destination::{ArchiveDestination, SinkConsumer},
};
use crate::{cli::SolidEntriesTransformStrategy, utils::GlobPatterns};
use pna::NormalEntry;
use std::{borrow::Cow, io};

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

/// Rewrites `source` into `destination` through `transform`, one entry at a time, and
/// publishes the destination only once the rewrite and the selector validation have succeeded.
pub(crate) fn execute_archive_transform(
    source: ArchiveSource,
    destination: ArchiveDestination,
    umask: Umask,
    password: Option<&[u8]>,
    strategy: SolidEntriesTransformStrategy,
    transform: impl EntryTransform,
) -> anyhow::Result<()> {
    destination.open_with(
        umask,
        RewriteEntries {
            source,
            password,
            strategy,
            transform,
        },
    )
}

struct RewriteEntries<'p, T> {
    source: ArchiveSource,
    password: Option<&'p [u8]>,
    strategy: SolidEntriesTransformStrategy,
    transform: T,
}

impl<T: EntryTransform> SinkConsumer for RewriteEntries<'_, T> {
    type Output = ();

    fn consume<W: io::Write>(mut self, mut writer: W) -> anyhow::Result<()> {
        let source = self.source.open()?;
        match self.strategy {
            SolidEntriesTransformStrategy::UnSolid => source.transform_entries(
                &mut writer,
                self.password,
                &mut self.transform,
                TransformStrategyUnSolid,
            ),
            SolidEntriesTransformStrategy::KeepSolid => source.transform_entries(
                &mut writer,
                self.password,
                &mut self.transform,
                TransformStrategyKeepSolid,
            ),
        }?;
        // A selector that matched nothing aborts here, before open_with publishes.
        if let Some(patterns) = self.transform.patterns() {
            patterns.ensure_all_matched()?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(target_family = "wasm"))]
mod tests {
    use super::*;
    use pna::{Archive, FileEntryBuilder};
    use std::{
        fs,
        path::{Path, PathBuf},
    };

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
            ArchiveSource::File(archive.to_path_buf()),
            ArchiveDestination::InPlace(archive.to_path_buf()),
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
