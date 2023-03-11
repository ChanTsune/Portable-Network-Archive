use super::Options;
use glob::Pattern;
use libpna::{Decoder, Entry};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn list_archive<A: AsRef<Path>, F: AsRef<Path>>(
    archive: A,
    files: &[F],
    options: Options,
) -> io::Result<()> {
    let globs = files
        .iter()
        .map(|p| Pattern::new(&p.as_ref().to_string_lossy()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let file = File::open(archive)?;

    let decoder = Decoder::new();
    let mut reader = decoder.read_header(file)?;
    while let Some(item) = reader.read()? {
        let item = item.into_read_entry()?;
        let path = PathBuf::from(item.header().path().as_str());
        if !globs.is_empty() && !globs.iter().any(|glob| glob.matches_path(&path)) {
            if !options.quiet && options.verbose {
                println!("Skip: {}", item.header().path())
            }
            continue;
        }
        println!("{}", path.display())
    }
    Ok(())
}
