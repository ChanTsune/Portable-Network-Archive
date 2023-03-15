use crate::cli::{
    CipherAlgorithmArgs, CipherMode, CompressionAlgorithmArgs, CreateArgs, Verbosity,
};
use crate::command::{ask_password, check_password};
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use libpna::{Encoder, Entry, EntryBuilder};
use rayon::ThreadPoolBuilder;
use std::path::PathBuf;
use std::time::Instant;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

pub(crate) fn create_archive(args: CreateArgs, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let start = Instant::now();
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let archive = args.file.archive;
    if !args.overwrite && archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is alrady exists", archive.display()),
        ));
    }
    if verbosity != Verbosity::Quite {
        println!("Create an archive: {}", archive.display());
    }
    let mut target_items = vec![];
    for p in args.file.files {
        collect_items(&mut target_items, p.as_ref(), args.recursive)?;
    }

    let progress_bar = ProgressBar::new(target_items.len() as u64)
        .with_style(ProgressStyle::default_bar().progress_chars("=> "));

    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(archive)?;

    let (tx, rx) = std::sync::mpsc::channel();
    let encoder = Encoder::new();
    let mut writer = encoder.write_header(file)?;

    for file in target_items {
        let compression = args.compression.clone();
        let cipher = args.cipher.clone();
        let password = password.clone();
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            tx.send(write_internal(
                &file,
                compression,
                cipher,
                password,
                verbosity,
            ))
            .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
        });
    }

    drop(tx);
    for item in rx {
        writer.add_entry(item?)?;
        progress_bar.inc(1);
    }

    writer.finalize()?;

    progress_bar.finish_and_clear();

    if verbosity != Verbosity::Quite {
        println!(
            "Successfully created an archive in {}",
            HumanDuration(start.elapsed())
        );
    }
    Ok(())
}

fn collect_items(result: &mut Vec<PathBuf>, path: &Path, recursive: bool) -> io::Result<()> {
    if path.is_dir() {
        if recursive {
            for p in fs::read_dir(path)? {
                collect_items(result, &p?.path(), recursive)?;
            }
        }
    } else if path.is_file() {
        result.push(path.to_path_buf());
    }
    Ok(())
}

fn write_internal(
    path: &Path,
    compression: CompressionAlgorithmArgs,
    cipher: CipherAlgorithmArgs,
    password: Option<String>,
    verbosity: Verbosity,
) -> io::Result<impl Entry> {
    if verbosity == Verbosity::Verbose {
        println!("Adding: {}", path.display());
    }
    if path.is_file() {
        let mut option_builder = libpna::WriteOptionBuilder::default();
        if compression.store {
            option_builder.compression(libpna::Compression::No);
        } else if let Some(xz_level) = compression.xz {
            option_builder.compression(libpna::Compression::XZ);
            if let Some(level) = xz_level {
                option_builder.compression_level(libpna::CompressionLevel::from(level));
            }
        } else if let Some(zstd_level) = compression.zstd {
            option_builder.compression(libpna::Compression::ZStandard);
            if let Some(level) = zstd_level {
                option_builder.compression_level(libpna::CompressionLevel::from(level));
            }
        } else if let Some(deflate_level) = compression.deflate {
            option_builder.compression(libpna::Compression::Deflate);
            if let Some(level) = deflate_level {
                option_builder.compression_level(libpna::CompressionLevel::from(level));
            }
        } else {
            option_builder.compression(libpna::Compression::ZStandard);
        }
        option_builder
            .encryption(if password.is_some() {
                if cipher.aes.is_some() {
                    libpna::Encryption::Aes
                } else if cipher.camellia.is_some() {
                    libpna::Encryption::Camellia
                } else {
                    libpna::Encryption::Aes
                }
            } else {
                libpna::Encryption::No
            })
            .cipher_mode(
                match match (cipher.aes, cipher.camellia) {
                    (Some(mode), _) | (_, Some(mode)) => mode.unwrap_or_default(),
                    (None, None) => CipherMode::default(),
                } {
                    CipherMode::Cbc => libpna::CipherMode::CBC,
                    CipherMode::Ctr => libpna::CipherMode::CTR,
                },
            )
            .password(password);
        let mut entry = EntryBuilder::new_file(path.into(), option_builder.build())?;
        entry.write_all(&fs::read(path)?)?;
        return entry.build();
    }
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Currently not a regular file is not supported.",
    ))
}
