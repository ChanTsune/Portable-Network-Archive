use super::Options;
use glob::Pattern;
use libpna::{Decoder, Entry, ReadOptionBuilder};
use rayon::ThreadPoolBuilder;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{fs, io};

pub(crate) fn extract_archive<A: AsRef<Path>, F: AsRef<Path>>(
    archive: A,
    files: &[F],
    options: Options,
) -> io::Result<()> {
    if !options.quiet {
        println!("Extract archive {}", archive.as_ref().display());
    }
    let globs = files
        .iter()
        .map(|p| Pattern::new(&p.as_ref().to_string_lossy()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let file = File::open(archive)?;

    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let (tx, rx) = std::sync::mpsc::channel();
    let decoder = Decoder::new();
    let mut reader = decoder.read_header(file)?;
    while let Some(item) = reader.read()? {
        let item_path = PathBuf::from(item.header().path().as_ref());
        if !globs.is_empty() && !globs.iter().any(|glob| glob.matches_path(&item_path)) {
            if !options.quiet && options.verbose {
                println!("Skip: {}", item.header().path())
            }
            continue;
        }
        let tx = tx.clone();
        let options = options.clone();
        pool.spawn_fifo(move || {
            tx.send(extract_entry(item_path, item, options)).unwrap();
        });
    }
    drop(tx);
    let _: Vec<_> = rx.into_iter().collect();
    if !options.quiet {
        println!("Successfully extracted an archive");
    }
    Ok(())
}

fn extract_entry(item_path: PathBuf, item: impl Entry, options: Options) -> io::Result<()> {
    let path = if let Some(out_dir) = &options.out_dir {
        out_dir.join(&item_path)
    } else {
        item_path.clone()
    };
    if path.exists() && !options.overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is alrady exists", path.display()),
        ));
    }
    if !options.quiet && options.verbose {
        println!("Extract: {}", item_path.display());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(&path)?;
    if !options.quiet && options.verbose {
        println!("start: {}", path.display())
    }
    let mut reader = item.to_reader({
        let mut builder = ReadOptionBuilder::new();
        if let Some(password) = options.password.flatten() {
            builder.password(password);
        }
        builder.build()
    })?;
    io::copy(&mut reader, &mut file)?;
    if !options.quiet && options.verbose {
        println!("end: {}", path.display())
    }
    Ok(())
}
