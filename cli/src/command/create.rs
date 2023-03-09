use super::{CipherMode, Options};
use libpna::{ArchiveWriter, Encoder};
use std::path::PathBuf;
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
    let mut target_items = vec![];
    for p in files {
        collect_items(&mut target_items, p.as_ref(), &options)?;
    }

    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(archive)?;

    let encoder = Encoder::new();
    let mut writer = encoder.write_header(file)?;

    for file in target_items {
        let file = file.as_ref();
        write_internal(&mut writer, file, &options)?;
    }

    writer.finalize()?;

    if !options.quiet {
        println!("Successfully created an archive");
    }
    Ok(())
}

fn collect_items(result: &mut Vec<PathBuf>, path: &Path, options: &Options) -> io::Result<()> {
    if path.is_dir() {
        if options.recursive {
            for p in fs::read_dir(path)? {
                collect_items(result, &p?.path(), options)?;
            }
        }
    } else if path.is_file() {
        result.push(path.to_path_buf());
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
    if path.is_file() {
        let mut item_option = libpna::Options::default();
        if options.store {
            item_option = item_option.compression(libpna::Compression::No);
        } else if let Some(lzma_level) = options.lzma {
            item_option = item_option.compression(libpna::Compression::XZ);
            if let Some(level) = lzma_level {
                item_option = item_option.compression_level(libpna::CompressionLevel::from(level))
            }
        } else if let Some(zstd_level) = options.zstd {
            item_option = item_option.compression(libpna::Compression::ZStandard);
            if let Some(level) = zstd_level {
                item_option = item_option.compression_level(libpna::CompressionLevel::from(level))
            }
        } else if let Some(deflate_level) = options.deflate {
            item_option = item_option.compression(libpna::Compression::Deflate);
            if let Some(level) = deflate_level {
                item_option = item_option.compression_level(libpna::CompressionLevel::from(level))
            }
        } else {
            item_option = item_option.compression(libpna::Compression::ZStandard);
        }
        item_option = item_option
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
        writer.start_file_with_options(path.into(), item_option)?;
        writer.write_all(&fs::read(path)?)?;
        writer.end_file()?;
    }
    Ok(())
}
