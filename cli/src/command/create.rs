use super::{CipherMode, Options};
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
    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
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
        let mut option_builder = libpna::WriteOptionBuilder::default();
        if options.store {
            option_builder.compression(libpna::Compression::No);
        } else if let Some(lzma_level) = options.lzma {
            option_builder.compression(libpna::Compression::XZ);
            if let Some(level) = lzma_level {
                option_builder.compression_level(libpna::CompressionLevel::from(level));
            }
        } else if let Some(zstd_level) = options.zstd {
            option_builder.compression(libpna::Compression::ZStandard);
            if let Some(level) = zstd_level {
                option_builder.compression_level(libpna::CompressionLevel::from(level));
            }
        } else if let Some(deflate_level) = options.deflate {
            option_builder.compression(libpna::Compression::Deflate);
            if let Some(level) = deflate_level {
                option_builder.compression_level(libpna::CompressionLevel::from(level));
            }
        } else {
            option_builder.compression(libpna::Compression::ZStandard);
        }
        option_builder
            .encryption(if let Some(Some(_)) = &options.password {
                if options.aes.is_some() {
                    libpna::Encryption::Aes
                } else if options.camellia.is_some() {
                    libpna::Encryption::Camellia
                } else {
                    libpna::Encryption::Aes
                }
            } else {
                libpna::Encryption::No
            })
            .cipher_mode(
                match match (options.aes, options.camellia) {
                    (Some(mode), _) | (_, Some(mode)) => mode.unwrap_or_default(),
                    (None, None) => CipherMode::default(),
                } {
                    CipherMode::Cbc => libpna::CipherMode::CBC,
                    CipherMode::Ctr => libpna::CipherMode::CTR,
                },
            )
            .password(options.password.clone().flatten());
        writer.start_file_with_options(path.into(), option_builder.build())?;
        writer.write_all(&fs::read(path)?)?;
        writer.end_file()?;
    }
    Ok(())
}
