use crate::cli::{ListArgs, Verbosity};
use glob::Pattern;
use libpna::{Decoder, ReadEntry};
use std::fs::File;
use std::io;
use std::path::PathBuf;

pub(crate) fn list_archive(args: ListArgs, verbosity: Verbosity) -> io::Result<()> {
    let globs = args
        .file
        .files
        .iter()
        .map(|p| Pattern::new(&p.to_string_lossy()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let file = File::open(args.file.archive)?;

    let decoder = Decoder::new();
    let mut reader = decoder.read_header(file)?;
    while let Some(item) = reader.read()? {
        let path = PathBuf::from(item.header().path().as_str());
        if !globs.is_empty() && !globs.iter().any(|glob| glob.matches_path(&path)) {
            if verbosity == Verbosity::Verbose {
                println!("Skip: {}", item.header().path())
            }
            continue;
        }
        println!("{}", path.display())
    }
    Ok(())
}
