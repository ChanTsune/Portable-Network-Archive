use crate::Options;
use libpna::Decoder;
use std::fs::File;
use std::path::Path;
use std::{fs, io};

pub(crate) fn extract_archive<A: AsRef<Path>, F: AsRef<Path>>(
    archive: A,
    files: &[F],
    options: Options,
) -> io::Result<()> {
    if !options.quiet {
        println!("Extract archive {}", archive.as_ref().display());
    }

    let files = files.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    let file = File::open(archive)?;
    let decoder = Decoder::new();
    let mut reader = decoder.read_header(file)?;
    while let Some(mut item) = reader.read(None)? {
        let path = Path::new(item.path());
        if !files.is_empty() {
            if !files.contains(&path) {
                if !options.quiet && options.verbose {
                    println!("Skip: {}", item.path())
                }
                continue;
            }
        }
        if path.exists() && options.overwrite {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} is alrady exists", path.display()),
            ));
        }
        if !options.quiet && options.verbose {
            println!("Extract: {}", path.display());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        io::copy(&mut item, &mut file)?;
    }
    if !options.quiet {
        println!("Successfully extracted an archive");
    }
    Ok(())
}
