use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process,
};

use clap::{CommandFactory, Parser, ValueEnum};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.command {
        Command::Mangen(args) => mangen(args),
        Command::Docgen(args) => docgen(args),
        Command::Tar2pna(args) => tar2pna(args),
        Command::Zip2pna(args) => zip2pna(args),
    }
}

#[derive(Parser)]
#[command(name = "xtask", about = "Development tasks for PNA")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Generate man pages for the CLI
    Mangen(MangenArgs),
    /// Generate markdown documentation for the CLI
    Docgen(DocgenArgs),
    /// Convert tar archive to PNA format
    Tar2pna(Tar2pnaArgs),
    /// Convert ZIP archive to PNA format
    Zip2pna(Zip2pnaArgs),
}

#[derive(Parser)]
struct MangenArgs {
    /// Output directory for man pages
    #[arg(short, long, default_value = "target/man")]
    output: PathBuf,
}

#[derive(Parser)]
struct DocgenArgs {
    /// Output file path for markdown documentation
    #[arg(short, long, default_value = "target/doc/pna.md")]
    output: PathBuf,
}

#[derive(Parser)]
struct Tar2pnaArgs {
    /// Input tar archive path (.tar, .tar.gz, .tar.bz2, .tar.xz, .tar.lzma, .tar.Z)
    input: PathBuf,
    /// Output PNA archive path (defaults to input stem with .pna extension)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Password for PNA encryption (AES-256-CTR)
    #[arg(long)]
    password: Option<String>,
    /// PNA compression method
    #[arg(short, long, default_value = "none")]
    compression: CompressionMethod,
}

#[derive(Parser)]
struct Zip2pnaArgs {
    /// Input ZIP archive path (.zip)
    input: PathBuf,
    /// Output PNA archive path (defaults to input stem with .pna extension)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Password for PNA encryption (AES-256-CTR)
    #[arg(long)]
    password: Option<String>,
    /// PNA compression method
    #[arg(short, long, default_value = "none")]
    compression: CompressionMethod,
}

#[derive(Copy, Clone, Debug, Default, ValueEnum)]
enum CompressionMethod {
    #[default]
    None,
    #[value(alias = "store")]
    Store,
    #[value(alias = "deflate")]
    Zlib,
    #[value(alias = "zstandard")]
    Zstd,
    Xz,
}

fn mangen(args: MangenArgs) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = &args.output;
    fs::create_dir_all(out_dir)?;

    // Get the CLI command and rename to match the binary name
    let cmd = portable_network_archive::cli::Cli::command().name("pna");

    // Use clap_mangen::generate_to which properly handles subcommands
    // and global argument propagation
    clap_mangen::generate_to(cmd, out_dir)?;

    eprintln!("Man pages generated in: {}", out_dir.display());
    Ok(())
}

fn docgen(args: DocgenArgs) -> Result<(), Box<dyn std::error::Error>> {
    let out_path = &args.output;

    // Get the CLI command and rename to match the binary name
    let cmd = portable_network_archive::cli::Cli::command().name("pna");

    // Generate markdown documentation
    let markdown = clap_markdown::help_markdown_command(&cmd);

    // Create a parent directory if it doesn't exist
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(out_path, &markdown)?;

    eprintln!("Markdown documentation generated: {}", out_path.display());
    Ok(())
}

fn tar2pna(args: Tar2pnaArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut child) = open_tar_reader(&args.input)?;
    let mut tar = tar::Archive::new(reader);

    let output_path = args.output.unwrap_or_else(|| {
        let stem = tar_stem(&args.input);
        args.input.with_file_name(format!("{stem}.pna"))
    });
    let write_options = build_write_options(args.compression, args.password.as_deref());

    // Capture all fallible work so child.wait() always runs even on early errors
    let convert_result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let output_file = File::create(&output_path)?;
        let mut archive = libpna::Archive::write_header(output_file)?;
        for entry_result in tar.entries()? {
            let mut entry = entry_result?;
            convert_entry(&mut entry, &mut archive, &write_options)?;
        }
        archive.finalize()?;
        Ok(())
    })();

    // Close the pipe read end so a blocked decompressor gets SIGPIPE instead of deadlocking
    drop(tar);

    // Always wait on decompressor child to avoid zombies, even if conversion failed
    if let Some(ref mut child) = child {
        match child.wait() {
            Ok(status) if !status.success() && convert_result.is_ok() => {
                return Err(format!("decompressor exited with {status}").into());
            }
            Err(e) if convert_result.is_ok() => return Err(e.into()),
            _ => {}
        }
    }
    convert_result?;

    eprintln!("PNA archive created: {}", output_path.display());
    Ok(())
}

type TarReader = (Box<dyn Read>, Option<process::Child>);

fn open_tar_reader(path: &Path) -> Result<TarReader, Box<dyn std::error::Error>> {
    let name = path.to_string_lossy();

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let file = File::open(path)?;
        Ok((Box::new(flate2::read::GzDecoder::new(file)), None))
    } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
        let file = File::open(path)?;
        Ok((Box::new(bzip2::read::BzDecoder::new(file)), None))
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        let file = File::open(path)?;
        Ok((Box::new(liblzma::read::XzDecoder::new(file)), None))
    } else if name.ends_with(".tar.lzma") {
        let file = File::open(path)?;
        let stream = liblzma::stream::Stream::new_lzma_decoder(u64::MAX)?;
        Ok((
            Box::new(liblzma::read::XzDecoder::new_stream(file, stream)),
            None,
        ))
    } else if name.ends_with(".tar.Z") {
        // No pure-Rust .Z (LZW compress) decoder available
        let (stdout, child) = spawn_decompressor("gzip", path)?;
        Ok((Box::new(stdout), Some(child)))
    } else if name.ends_with(".tar") {
        let file = File::open(path)?;
        Ok((Box::new(file), None))
    } else {
        Err(format!("unsupported archive format: {name}").into())
    }
}

fn spawn_decompressor(
    cmd: &str,
    path: &Path,
) -> Result<(process::ChildStdout, process::Child), Box<dyn std::error::Error>> {
    let mut child = process::Command::new(cmd)
        .args(["-d", "-c"])
        .arg(path)
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn {cmd}: {e}"))?;
    let stdout = child.stdout.take().expect("stdout piped");
    Ok((stdout, child))
}

/// Extract the archive stem, stripping `.tar.*` suffixes.
fn tar_stem(path: &Path) -> String {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    for suffix in [
        ".tar.gz",
        ".tgz",
        ".tar.bz2",
        ".tbz2",
        ".tar.xz",
        ".txz",
        ".tar.lzma",
        ".tar.Z",
        ".tar",
    ] {
        if let Some(stem) = name.strip_suffix(suffix) {
            return stem.to_string();
        }
    }
    name
}

fn build_write_options(method: CompressionMethod, password: Option<&str>) -> libpna::WriteOptions {
    let compression = match method {
        CompressionMethod::None | CompressionMethod::Store => libpna::Compression::No,
        CompressionMethod::Zlib => libpna::Compression::Deflate,
        CompressionMethod::Zstd => libpna::Compression::ZStandard,
        CompressionMethod::Xz => libpna::Compression::XZ,
    };
    if let Some(pw) = password {
        libpna::WriteOptions::builder()
            .compression(compression)
            .encryption(libpna::Encryption::Aes)
            .cipher_mode(libpna::CipherMode::CTR)
            .password(Some(pw))
            .build()
    } else if matches!(compression, libpna::Compression::No) {
        libpna::WriteOptions::store()
    } else {
        libpna::WriteOptions::builder()
            .compression(compression)
            .build()
    }
}

fn convert_entry<R: Read, W: Write>(
    entry: &mut tar::Entry<'_, R>,
    archive: &mut libpna::Archive<W>,
    write_options: &libpna::WriteOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let header = entry.header();
    let entry_type = header.entry_type();
    let path = entry.path()?.to_string_lossy().to_string();
    let mtime = header.mtime().unwrap_or_else(|e| {
        eprintln!("warning: {path}: failed to read mtime ({e}), defaulting to 0");
        0
    });
    let mtime_duration = libpna::Duration::new(mtime as i64, 0);
    let permission = build_permission(header, &path);

    if entry_type.is_dir() {
        let mut builder = libpna::EntryBuilder::new_dir(path.as_str().into());
        builder.modified(mtime_duration);
        builder.permission(permission);
        archive.add_entry(builder.build()?)?;
    } else if entry_type.is_symlink() {
        let link = entry
            .link_name()?
            .ok_or("symlink missing link name")?
            .to_string_lossy()
            .to_string();
        let mut builder = libpna::EntryBuilder::new_symlink(
            path.as_str().into(),
            libpna::EntryReference::from(link.as_str()),
        )?;
        builder.modified(mtime_duration);
        builder.permission(permission);
        archive.add_entry(builder.build()?)?;
    } else if entry_type.is_hard_link() {
        let link = entry
            .link_name()?
            .ok_or("hardlink missing link name")?
            .to_string_lossy()
            .to_string();
        let mut builder = libpna::EntryBuilder::new_hard_link(
            path.as_str().into(),
            libpna::EntryReference::from(link.as_str()),
        )?;
        builder.modified(mtime_duration);
        builder.permission(permission);
        archive.add_entry(builder.build()?)?;
    } else if entry_type.is_file() {
        let mut builder =
            libpna::EntryBuilder::new_file(path.as_str().into(), write_options.clone())?;
        io::copy(entry, &mut builder)?;
        builder.modified(mtime_duration);
        builder.permission(permission);
        archive.add_entry(builder.build()?)?;
    } else {
        eprintln!(
            "warning: skipping unsupported entry type {:?}: {path}",
            entry_type
        );
    }

    Ok(())
}

fn build_permission(header: &tar::Header, path: &str) -> libpna::Permission {
    let uid = header.uid().unwrap_or_else(|e| {
        eprintln!("warning: {path}: failed to read uid ({e}), defaulting to 0");
        0
    });
    let gid = header.gid().unwrap_or_else(|e| {
        eprintln!("warning: {path}: failed to read gid ({e}), defaulting to 0");
        0
    });
    let mode = (header.mode().unwrap_or_else(|e| {
        eprintln!("warning: {path}: failed to read mode ({e}), defaulting to 0o644");
        0o644
    }) & 0o7777) as u16;
    let uname = match header.username() {
        Ok(Some(name)) => name.to_string(),
        Ok(None) => String::new(),
        Err(e) => {
            eprintln!("warning: {path}: username is not valid UTF-8 ({e})");
            String::new()
        }
    };
    let gname = match header.groupname() {
        Ok(Some(name)) => name.to_string(),
        Ok(None) => String::new(),
        Err(e) => {
            eprintln!("warning: {path}: groupname is not valid UTF-8 ({e})");
            String::new()
        }
    };
    libpna::Permission::new(uid, uname, gid, gname, mode)
}

fn zip2pna(args: Zip2pnaArgs) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(&args.input)?;
    let mut zip = zip::ZipArchive::new(file)?;

    let output_path = args.output.unwrap_or_else(|| {
        let stem = zip_stem(&args.input);
        args.input.with_file_name(format!("{stem}.pna"))
    });
    let write_options = build_write_options(args.compression, args.password.as_deref());

    let output_file = File::create(&output_path)?;
    let mut archive = libpna::Archive::write_header(output_file)?;

    for i in 0..zip.len() {
        let mut entry = match &args.password {
            Some(password) => zip.by_index_decrypt(i, password.as_bytes())?,
            None => zip.by_index(i)?,
        };
        convert_zip_entry(&mut entry, &mut archive, &write_options)?;
    }

    archive.finalize()?;
    eprintln!("PNA archive created: {}", output_path.display());
    Ok(())
}

fn zip_stem(path: &Path) -> String {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    name.strip_suffix(".zip").unwrap_or(&name).to_string()
}

fn convert_zip_entry<R: Read + io::Seek, W: Write>(
    entry: &mut zip::read::ZipFile<'_, R>,
    archive: &mut libpna::Archive<W>,
    write_options: &libpna::WriteOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = entry.name().to_string();
    let mtime = zip_last_modified(entry, &path);
    let permission = build_zip_permission(entry);

    if entry.is_dir() {
        let mut builder = libpna::EntryBuilder::new_dir(path.as_str().into());
        builder.modified(mtime);
        if let Some(perm) = permission {
            builder.permission(perm);
        }
        archive.add_entry(builder.build()?)?;
    } else if entry.is_symlink() {
        let mut target = String::new();
        entry.read_to_string(&mut target)?;
        let mut builder = libpna::EntryBuilder::new_symlink(
            path.as_str().into(),
            libpna::EntryReference::from(target.as_str()),
        )?;
        builder.modified(mtime);
        if let Some(perm) = permission {
            builder.permission(perm);
        }
        archive.add_entry(builder.build()?)?;
    } else if entry.is_file() {
        let mut builder =
            libpna::EntryBuilder::new_file(path.as_str().into(), write_options.clone())?;
        io::copy(entry, &mut builder)?;
        builder.modified(mtime);
        if let Some(perm) = permission {
            builder.permission(perm);
        }
        archive.add_entry(builder.build()?)?;
    } else {
        eprintln!("warning: skipping unsupported entry: {path}");
    }

    Ok(())
}

fn zip_last_modified<R: Read + io::Seek>(
    entry: &zip::read::ZipFile<'_, R>,
    path: &str,
) -> libpna::Duration {
    // ExtendedTimestamp carries UTC, avoiding MS-DOS timestamp's timezone ambiguity
    for field in entry.extra_data_fields() {
        if let zip::extra_fields::ExtraField::ExtendedTimestamp(ts) = field
            && let Some(mtime) = ts.mod_time()
        {
            return libpna::Duration::new(mtime as i64, 0);
        }
    }
    // MS-DOS timestamps have no timezone; assume_utc() is lossy but consistent
    let Some(dt) = entry.last_modified() else {
        return libpna::Duration::new(0, 0);
    };
    match time::PrimitiveDateTime::try_from(dt) {
        Ok(pdt) => libpna::Duration::new(pdt.assume_utc().unix_timestamp(), 0),
        Err(e) => {
            eprintln!("warning: {path}: invalid timestamp ({e}), defaulting to 0");
            libpna::Duration::new(0, 0)
        }
    }
}

fn build_zip_permission<R: Read + io::Seek>(
    entry: &zip::read::ZipFile<'_, R>,
) -> Option<libpna::Permission> {
    entry.unix_mode().map(|mode| {
        let mode_bits = (mode & 0o7777) as u16;
        libpna::Permission::new(0, String::new(), 0, String::new(), mode_bits)
    })
}
