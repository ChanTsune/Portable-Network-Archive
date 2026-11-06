use super::ArchiveSource;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum ArchiveDestination {
    Stdout,
    CreateNew(PathBuf),
    Replace(PathBuf),
    InPlace(PathBuf),
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
mod tests {
    use super::*;

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
}
