use crate::Options;
use libpna::{ArchiveWriter, Encoder};
use std::{
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
    let file = File::create(archive)?;

    let encoder = Encoder::new();
    let mut writer = encoder.write_header(file)?;

    for file in files {
        let file = file.as_ref();
        write_internal(&mut writer, file, archive, &options)?;
    }

    writer.finalize()?;

    if !options.quiet {
        println!("Successfully created an archive");
    }
    Ok(())
}

fn write_internal<W: Write>(
    writer: &mut ArchiveWriter<W>,
    path: &Path,
    ignore: &Path,
    options: &Options,
) -> io::Result<()> {
    if path.canonicalize()? == ignore.canonicalize()? {
        return Ok(());
    }
    if !options.quiet && options.verbose {
        println!("Adding: {}", path.display());
    }
    if path.is_dir() {
        if options.recursive {
            for i in fs::read_dir(path)? {
                write_internal(writer, &i?.path(), ignore, options)?;
            }
        }
    } else if path.is_file() {
        let item_option = libpna::Options::default().compression(if options.store {
            libpna::Compression::No
        } else if let Some(lzma_level) = options.lzma {
            libpna::Compression::XZ
        } else if let Some(zstd_level) = options.zstd {
            libpna::Compression::ZStandard
        } else {
            libpna::Compression::No
        });
        writer.start_file_with_options(path.as_os_str().to_string_lossy().as_ref(), item_option)?;
        writer.write_all(&fs::read(path)?)?;
        writer.end_file()?;
    }
    Ok(())
}
