use crate::Options;
use libpna::Encoder;
use std::{fs::File, io, path::Path};

pub(crate) fn create_archive<A: AsRef<Path>, F: AsRef<Path>>(
    archive: A,
    files: &[F],
    options: Options,
) -> io::Result<()> {
    let archive = archive.as_ref();
    if !options.overwrite && archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is alrady exists", archive.display()),
        ));
    }
    if !options.quiet {
        println!("Create an archive: {}", archive.display());
    }
    let file = File::create(archive)?;

    let encoder = Encoder::new();
    let mut writer = encoder.write_header(file)?;

    for file in files {
        let file = file.as_ref();
        if !options.quiet && options.verbose {
            println!("Adding: {}", file.display());
        }
    }

    writer.finalize()?;

    if !options.quiet {
        println!("Successfully created an archive");
    }
    Ok(())
}
