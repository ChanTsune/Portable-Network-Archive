use crate::Options;
use libpna::Decoder;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn list_archive<A: AsRef<Path>, F: AsRef<Path>>(
    archive: A,
    files: &[F],
    options: Options,
) -> io::Result<()> {
    let files = files.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    let file = File::open(archive)?;

    let decoder = Decoder::new();
    let mut reader = decoder.read_header(file)?;
    while let Some(item) = reader.read(options.password.clone().flatten().as_deref())? {
        let path = PathBuf::from(item.path());
        if !files.is_empty() && !files.contains(&path.as_path()) {
            if !options.quiet && options.verbose {
                println!("Skip: {}", item.path())
            }
            continue;
        }
        println!("{}", path.display())
    }
    Ok(())
}
