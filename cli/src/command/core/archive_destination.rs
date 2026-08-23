use super::{ArchiveSource, StagedArchive, Umask};
use std::{
    fmt, fs,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ArchiveDestination {
    Stdout,
    CreateNew(PathBuf),
    Replace(PathBuf),
    InPlace(PathBuf),
}

impl fmt::Display for ArchiveDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdout => f.write_str("standard output"),
            Self::CreateNew(path) | Self::Replace(path) | Self::InPlace(path) => {
                path.display().fmt(f)
            }
        }
    }
}

/// Continuation run with the opened destination's writer. The writer's concrete type is
/// only known once the destination is opened, so the write path is expressed as a generic
/// method and monomorphizes per writer kind instead of dispatching on every write.
///
/// [`ArchiveDestination::open_with`] publishes the destination after `consume` returns
/// successfully, so a consumer cannot forget to commit; a consumer that must hold back
/// publication (e.g. a selector that matched nothing) returns an error instead.
pub(crate) trait SinkConsumer {
    type Output;
    fn consume<W: Write>(self, writer: W) -> anyhow::Result<Self::Output>;
}

impl ArchiveDestination {
    /// Opens the destination, lets `consumer` write the archive, and publishes it once the
    /// consumer succeeds. Both filesystem destinations write through a plain [`fs::File`],
    /// so the write path monomorphizes twice: standard output and file.
    pub(crate) fn open_with<C: SinkConsumer>(
        self,
        umask: Umask,
        consumer: C,
    ) -> anyhow::Result<C::Output> {
        match self {
            Self::Stdout => {
                let mut stdout = io::stdout().lock();
                let output = consumer.consume(&mut stdout)?;
                stdout.flush()?;
                Ok(output)
            }
            Self::CreateNew(path) => {
                let mut sink = CreateNewArchive::new(path)?;
                let output = consumer.consume(sink.file_mut())?;
                sink.commit()?;
                Ok(output)
            }
            Self::Replace(path) | Self::InPlace(path) => {
                let mut staged = StagedArchive::new(path, umask)?;
                let output = consumer.consume(staged.as_file_mut())?;
                staged.commit()?;
                Ok(output)
            }
        }
    }
}

pub(crate) struct CreateNewArchive {
    path: PathBuf,
    file: fs::File,
    committed: bool,
}

impl CreateNewArchive {
    fn new(path: PathBuf) -> io::Result<Self> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self {
            path,
            file,
            committed: false,
        })
    }

    fn file_mut(&mut self) -> &mut fs::File {
        &mut self.file
    }

    fn commit(mut self) -> io::Result<()> {
        self.file.sync_all()?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for CreateNewArchive {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Err(error) = fs::remove_file(&self.path)
            && error.kind() != io::ErrorKind::NotFound
        {
            log::warn!(
                "failed to clean up incomplete archive '{}': {error}",
                self.path.display()
            );
        }
    }
}

pub(crate) fn resolve_transform_destination(
    source: &ArchiveSource,
    output: Option<PathBuf>,
    overwrite: bool,
) -> anyhow::Result<ArchiveDestination> {
    if let Some(path) = output {
        return Ok(if overwrite {
            ArchiveDestination::Replace(path)
        } else {
            ArchiveDestination::CreateNew(path)
        });
    }

    match (source, overwrite) {
        (_, false) => Ok(ArchiveDestination::Stdout),
        (ArchiveSource::File(path), true) => Ok(ArchiveDestination::InPlace(path.clone())),
        (ArchiveSource::Stdin, true) => anyhow::bail!(
            "--overwrite requires a filesystem destination; specify --output PATH or provide the source with --file"
        ),
    }
}

pub(crate) fn resolve_create_destination(
    file: Option<PathBuf>,
    overwrite: bool,
) -> anyhow::Result<ArchiveDestination> {
    match (file, overwrite) {
        (None, false) => Ok(ArchiveDestination::Stdout),
        (Some(path), false) => Ok(ArchiveDestination::CreateNew(path)),
        (Some(path), true) => Ok(ArchiveDestination::Replace(path)),
        (None, true) => {
            anyhow::bail!("--overwrite requires --file PATH when creating an archive")
        }
    }
}

#[cfg(test)]
#[cfg(not(target_family = "wasm"))]
mod tests {
    use super::*;

    /// Writes `payload`, then reports the outcome a selector validation would have.
    struct WriteAndValidate<'a> {
        payload: &'a [u8],
        valid: bool,
    }

    impl SinkConsumer for WriteAndValidate<'_> {
        type Output = ();

        fn consume<W: Write>(self, mut writer: W) -> anyhow::Result<()> {
            writer.write_all(self.payload)?;
            anyhow::ensure!(self.valid, "selector matched nothing");
            Ok(())
        }
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("pna_archive_destination_test")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn file_source() -> ArchiveSource {
        ArchiveSource::File(PathBuf::from("source.pna"))
    }

    #[test]
    fn transform_destination_covers_the_contract_matrix() {
        let output = || Some(PathBuf::from("output.pna"));
        let cases = [
            (
                ArchiveSource::Stdin,
                None,
                false,
                Ok(ArchiveDestination::Stdout),
            ),
            (file_source(), None, false, Ok(ArchiveDestination::Stdout)),
            (
                file_source(),
                None,
                true,
                Ok(ArchiveDestination::InPlace(PathBuf::from("source.pna"))),
            ),
            (ArchiveSource::Stdin, None, true, Err(())),
            (
                ArchiveSource::Stdin,
                output(),
                false,
                Ok(ArchiveDestination::CreateNew(PathBuf::from("output.pna"))),
            ),
            (
                file_source(),
                output(),
                false,
                Ok(ArchiveDestination::CreateNew(PathBuf::from("output.pna"))),
            ),
            (
                ArchiveSource::Stdin,
                output(),
                true,
                Ok(ArchiveDestination::Replace(PathBuf::from("output.pna"))),
            ),
            (
                file_source(),
                output(),
                true,
                Ok(ArchiveDestination::Replace(PathBuf::from("output.pna"))),
            ),
        ];

        for (source, output, overwrite, expected) in cases {
            let actual = resolve_transform_destination(&source, output, overwrite);
            match expected {
                Ok(destination) => assert_eq!(actual.unwrap(), destination),
                Err(()) => assert_eq!(
                    actual.unwrap_err().to_string(),
                    "--overwrite requires a filesystem destination; specify --output PATH or provide the source with --file"
                ),
            }
        }
    }

    #[test]
    fn create_destination_covers_its_contract_matrix() {
        let file = || Some(PathBuf::from("archive.pna"));
        let cases = [
            (None, false, Ok(ArchiveDestination::Stdout)),
            (
                file(),
                false,
                Ok(ArchiveDestination::CreateNew(PathBuf::from("archive.pna"))),
            ),
            (
                file(),
                true,
                Ok(ArchiveDestination::Replace(PathBuf::from("archive.pna"))),
            ),
            (None, true, Err(())),
        ];

        for (file, overwrite, expected) in cases {
            let actual = resolve_create_destination(file, overwrite);
            match expected {
                Ok(destination) => assert_eq!(actual.unwrap(), destination),
                Err(()) => assert_eq!(
                    actual.unwrap_err().to_string(),
                    "--overwrite requires --file PATH when creating an archive"
                ),
            }
        }
    }

    #[test]
    fn create_new_never_clobbers_an_existing_file() {
        let path = test_dir("no_clobber").join("archive.pna");
        fs::write(&path, b"original").unwrap();

        let error = ArchiveDestination::CreateNew(path.clone())
            .open_with(
                Umask::new(0o022),
                WriteAndValidate {
                    payload: b"clobbered",
                    valid: true,
                },
            )
            .err()
            .unwrap();

        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(path).unwrap(), b"original");
    }

    #[test]
    fn failed_create_new_validation_removes_the_partial_archive() {
        let path = test_dir("create_new_validation").join("archive.pna");

        let result = ArchiveDestination::CreateNew(path.clone()).open_with(
            Umask::new(0o022),
            WriteAndValidate {
                payload: b"partial",
                valid: false,
            },
        );

        assert!(result.is_err());
        assert!(!path.exists());
    }

    #[test]
    fn committed_create_new_keeps_the_completed_archive() {
        let path = test_dir("create_new_commit").join("archive.pna");

        ArchiveDestination::CreateNew(path.clone())
            .open_with(
                Umask::new(0o022),
                WriteAndValidate {
                    payload: b"complete",
                    valid: true,
                },
            )
            .unwrap();

        assert_eq!(fs::read(path).unwrap(), b"complete");
    }

    #[test]
    fn replace_and_in_place_remain_transactional_until_publish() {
        for (name, replace) in [("replace", true), ("in_place", false)] {
            let path = test_dir(name).join("archive.pna");
            fs::write(&path, b"original").unwrap();
            let destination = if replace {
                ArchiveDestination::Replace(path.clone())
            } else {
                ArchiveDestination::InPlace(path.clone())
            };

            let result = destination.open_with(
                Umask::new(0o022),
                WriteAndValidate {
                    payload: b"partial",
                    valid: false,
                },
            );

            assert!(result.is_err());
            assert_eq!(fs::read(path).unwrap(), b"original");
        }
    }
}
