use crate::Options;
use libpna::{ArchiveWriter, Encoder};
use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

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
    let temp_dir = env::temp_dir();
    fs::create_dir_all(&temp_dir)?;
    let temp_file_path = temp_dir.join(archive.file_name().unwrap());

    let file = File::create(&temp_file_path)?;

    let encoder = Encoder::new();
    let mut writer = encoder.write_header(file)?;

    for file in files {
        let file = file.as_ref();
        write_internal(&mut writer, file, &options)?;
    }

    writer.finalize()?;

    fs::rename(temp_file_path, archive)?;

    if !options.quiet {
        println!("Successfully created an archive");
    }
    Ok(())
}

fn write_internal<W: Write>(
    writer: &mut ArchiveWriter<W>,
    path: &Path,
    options: &Options,
) -> io::Result<()> {
    if !options.quiet && options.verbose {
        println!("Adding: {}", path.display());
    }
    if path.is_dir() {
        if options.recursive {
            for i in fs::read_dir(path)? {
                write_internal(writer, &i?.path(), options)?;
            }
        }
    } else if path.is_file() {
        writer.start_file(path.as_os_str().to_string_lossy().as_ref())?;
        writer.write_all(&fs::read(path)?)?;
        writer.end_file()?;
    }
    Ok(())
}
