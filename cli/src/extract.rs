use crate::Options;
use libpna::Decoder;
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

    let files = files.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    let file = File::open(archive)?;

    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let (tx, rx) = std::sync::mpsc::channel();
    let decoder = Decoder::new();
    let mut reader = decoder.read_header(file)?;
    while let Some(mut item) = reader.read(options.password.clone().flatten().as_deref())? {
        let path = PathBuf::from(item.path());
        if !files.is_empty() {
            if !files.contains(&path.as_path()) {
                if !options.quiet && options.verbose {
                    println!("Skip: {}", item.path())
                }
                continue;
            }
        }
        if path.exists() && !options.overwrite {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} is alrady exists", path.display()),
            ));
        }
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            if !options.quiet && options.verbose {
                println!("Extract: {}", path.display());
            }
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut file = File::create(&path).unwrap();
            if !options.quiet && options.verbose {
                println!("start: {}", path.display())
            }
            io::copy(&mut item, &mut file).unwrap();
            if !options.quiet && options.verbose {
                println!("end: {}", path.display())
            }
            tx.send(()).unwrap();
        });
    }
    drop(tx);
    let _: Vec<_> = rx.into_iter().collect();
    if !options.quiet {
        println!("Successfully extracted an archive");
    }
    Ok(())
}
