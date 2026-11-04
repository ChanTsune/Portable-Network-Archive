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
    /// Input tar archive path (.tar, .tar.gz, .tar.bz2, .tar.xz, .tar.lzma, .tar.lz4, .tar.zst, .tar.Z)
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

    let cmd = portable_network_archive::cli::Cli::command().name("pna");

    // Use clap_mangen::generate_to which properly handles subcommands
    // and global argument propagation
    clap_mangen::generate_to(cmd, out_dir)?;

    eprintln!("Man pages generated in: {}", out_dir.display());
    Ok(())
}

fn docgen(args: DocgenArgs) -> Result<(), Box<dyn std::error::Error>> {
    let out_path = &args.output;

    let cmd = portable_network_archive::cli::Cli::command().name("pna");

    let markdown = clap_markdown::help_markdown_command(&cmd);

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
        pna_output_path(
            &args.input,
            &[
                ".tar.gz",
                ".tgz",
                ".tar.bz2",
                ".tbz2",
                ".tar.xz",
                ".txz",
                ".tar.lzma",
                ".tar.lz4",
                ".tar.zst",
                ".tzst",
                ".tar.Z",
                ".tar",
            ],
        )
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
    } else if name.ends_with(".tar.lz4") {
        let file = File::open(path)?;
        Ok((Box::new(lz4_flex::frame::FrameDecoder::new(file)), None))
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        let file = File::open(path)?;
        Ok((Box::new(zstd::Decoder::new(file)?), None))
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

fn pna_output_path(path: &Path, suffixes: &[&str]) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let stem = suffixes
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(&name);
    path.with_file_name(format!("{stem}.pna"))
}

fn build_write_options(method: CompressionMethod, password: Option<&str>) -> libpna::WriteOptions {
    let compression = match method {
        CompressionMethod::None | CompressionMethod::Store => libpna::Compression::NO,
        CompressionMethod::Zlib => libpna::Compression::DEFLATE,
        CompressionMethod::Zstd => libpna::Compression::ZSTANDARD,
        CompressionMethod::Xz => libpna::Compression::XZ,
    };
    if let Some(pw) = password {
        libpna::WriteOptions::builder()
            .compression(compression)
            .encryption(libpna::Encryption::AES)
            .cipher_mode(libpna::CipherMode::CTR)
            .password(Some(pw))
            .build()
    } else if matches!(compression, libpna::Compression::NO) {
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
    let path = entry.path()?;
    let mtime = header.mtime().unwrap_or_else(|e| {
        eprintln!(
            "warning: {}: failed to read mtime ({e}), defaulting to 0",
            path.display()
        );
        0
    });
    let mtime_duration = libpna::Duration::new(mtime as i64, 0);
    let metadata = tar_entry_metadata(header, &path, mtime_duration);
    let entry_name = libpna::EntryName::from_path_lossy_preserve_root(&path);

    if entry_type.is_dir() {
        let mut builder = libpna::DirEntryBuilder::new(entry_name);
        builder.metadata(metadata);
        archive.add_entry(builder.build()?)?;
    } else if entry_type.is_symlink() {
        let link = entry.link_name()?.ok_or("symlink missing link name")?;
        let mut builder = libpna::SymlinkEntryBuilder::new(
            entry_name,
            libpna::EntryReference::from_path_lossy_preserve_root(&link),
        )?;
        builder.metadata(metadata);
        archive.add_entry(builder.build()?)?;
    } else if entry_type.is_hard_link() {
        let link = entry.link_name()?.ok_or("hardlink missing link name")?;
        let mut builder = libpna::HardLinkEntryBuilder::new(
            entry_name,
            libpna::EntryReference::from_path_lossy_preserve_root(&link),
        )?;
        builder.metadata(metadata);
        archive.add_entry(builder.build()?)?;
    } else if entry_type.is_file() {
        let mut builder = libpna::FileEntryBuilder::new_with_options(entry_name, write_options)?;
        io::copy(entry, &mut builder)?;
        builder.metadata(metadata);
        archive.add_entry(builder.build()?)?;
    } else {
        eprintln!(
            "warning: skipping unsupported entry type {entry_type:?}: {}",
            path.display()
        );
    }

    Ok(())
}

fn owner_name_opt(s: &str) -> Option<libpna::OwnerUserName> {
    owner_name_bounded(s)
        .map(|t| libpna::OwnerUserName::new(t).expect("owner_name_bounded guarantees <= 255 bytes"))
}

fn owner_group_name_opt(s: &str) -> Option<libpna::OwnerGroupName> {
    owner_name_bounded(s).map(|t| {
        libpna::OwnerGroupName::new(t).expect("owner_name_bounded guarantees <= 255 bytes")
    })
}

fn owner_name_bounded(s: &str) -> Option<&str> {
    if s.is_empty() {
        return None;
    }
    const MAX: usize = u8::MAX as usize;
    if s.len() <= MAX {
        return Some(s);
    }
    Some(&s[..s.floor_char_boundary(MAX)])
}

fn tar_entry_metadata(
    header: &tar::Header,
    path: &Path,
    modified: libpna::Duration,
) -> libpna::Metadata {
    let path = path.display();
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
        Ok(Some(name)) => name,
        Ok(None) => "",
        Err(e) => {
            eprintln!("warning: {path}: username is not valid UTF-8 ({e})");
            ""
        }
    };
    let gname = match header.groupname() {
        Ok(Some(name)) => name,
        Ok(None) => "",
        Err(e) => {
            eprintln!("warning: {path}: groupname is not valid UTF-8 ({e})");
            ""
        }
    };
    libpna::Metadata::new()
        .with_modified(Some(modified))
        .with_owner_uid(Some(libpna::OwnerUid::from(uid)))
        .with_owner_gid(Some(libpna::OwnerGid::from(gid)))
        .with_permission_mode(Some(libpna::PermissionMode::from(mode)))
        .with_owner_user_name(owner_name_opt(uname))
        .with_owner_group_name(owner_group_name_opt(gname))
}

fn zip2pna(args: Zip2pnaArgs) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(&args.input)?;
    let mut zip = zip::ZipArchive::new(file)?;

    let output_path = args
        .output
        .unwrap_or_else(|| pna_output_path(&args.input, &[".zip"]));
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

fn convert_zip_entry<R: Read + io::Seek, W: Write>(
    entry: &mut zip::read::ZipFile<'_, R>,
    archive: &mut libpna::Archive<W>,
    write_options: &libpna::WriteOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = entry.name()?;
    let mtime = zip_last_modified(entry, &path);
    let entry_name = libpna::EntryName::from_utf8_preserve_root(&path);

    if entry.is_dir() {
        let mut builder = libpna::DirEntryBuilder::new(entry_name);
        builder.metadata(zip_entry_metadata(entry, mtime));
        archive.add_entry(builder.build()?)?;
    } else if entry.is_symlink() {
        let mut target = String::new();
        entry.read_to_string(&mut target)?;
        let mut builder = libpna::SymlinkEntryBuilder::new(
            entry_name,
            libpna::EntryReference::from_utf8_preserve_root(&target),
        )?;
        builder.metadata(zip_entry_metadata(entry, mtime));
        archive.add_entry(builder.build()?)?;
    } else if entry.is_file() {
        let mut builder = libpna::FileEntryBuilder::new_with_options(entry_name, write_options)?;
        io::copy(entry, &mut builder)?;
        builder.metadata(zip_entry_metadata(entry, mtime));
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

fn zip_entry_metadata<R: Read + io::Seek>(
    entry: &zip::read::ZipFile<'_, R>,
    modified: libpna::Duration,
) -> libpna::Metadata {
    let permission_mode = entry
        .unix_mode()
        .map(|mode| libpna::PermissionMode::from((mode & 0o7777) as u16));
    libpna::Metadata::new()
        .with_modified(Some(modified))
        .with_permission_mode(permission_mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_path_preserves_parent_and_strips_only_archive_suffix() {
        for (input, suffixes, expected) in [
            (
                "dir/日本語.tar.gz",
                &[".tar.gz", ".tar"][..],
                "dir/日本語.pna",
            ),
            ("dir/a.tar.tar", &[".tar"][..], "dir/a.tar.pna"),
            ("dir/a.zip", &[".zip"][..], "dir/a.pna"),
            ("dir/a.ZIP", &[".zip"][..], "dir/a.ZIP.pna"),
            ("dir/.zip", &[".zip"][..], "dir/.pna"),
        ] {
            assert_eq!(
                pna_output_path(Path::new(input), suffixes),
                Path::new(expected)
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn default_output_path_replaces_invalid_utf8_only_in_filename() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let input = Path::new(OsStr::from_bytes(b"dir\xff/name\xff.tar.gz"));
        let expected = Path::new(OsStr::from_bytes(b"dir\xff/name\xef\xbf\xbd.pna"));
        assert_eq!(pna_output_path(input, &[".tar.gz"]), expected);
    }

    #[test]
    fn tar_conversion_preserves_metadata_and_long_link_targets() {
        use std::io::Cursor;
        let target = format!("/root/../{}", "日本語/".repeat(30));
        let cases = [
            (
                "folder/",
                tar::EntryType::Directory,
                libpna::DataKind::DIRECTORY,
                "",
            ),
            (
                "日本語.txt",
                tar::EntryType::Regular,
                libpna::DataKind::FILE,
                "content\n",
            ),
            (
                "symlink",
                tar::EntryType::Symlink,
                libpna::DataKind::SYMBOLIC_LINK,
                target.as_str(),
            ),
            (
                "hardlink",
                tar::EntryType::Link,
                libpna::DataKind::HARD_LINK,
                target.as_str(),
            ),
        ];
        let mut tar = tar::Builder::new(Vec::new());
        for (name, kind, _, content) in cases {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(kind);
            header.set_uid(123);
            header.set_gid(456);
            header.set_mode(0o6754);
            header.set_mtime(1_700_000_000);
            header.set_username("owner").unwrap();
            header.set_groupname("group").unwrap();
            if kind.is_symlink() || kind.is_hard_link() {
                header.set_size(0);
                tar.append_link(&mut header, name, content).unwrap();
            } else {
                header.set_size(content.len() as u64);
                tar.append_data(&mut header, name, content.as_bytes())
                    .unwrap();
            }
        }
        let mut unsupported = tar::Header::new_gnu();
        unsupported.set_entry_type(tar::EntryType::Fifo);
        unsupported.set_size(0);
        unsupported.as_mut_bytes()[108..116].fill(b'x');
        tar.append_data(&mut unsupported, "fifo", io::empty())
            .unwrap();
        let mut tar = tar::Archive::new(Cursor::new(tar.into_inner().unwrap()));
        let mut bytes = Vec::new();
        let mut pna = libpna::Archive::write_header(&mut bytes).unwrap();
        for entry in tar.entries().unwrap() {
            convert_entry(
                &mut entry.unwrap(),
                &mut pna,
                &libpna::WriteOptions::store(),
            )
            .unwrap();
        }
        pna.finalize().unwrap();
        let options = libpna::ReadOptions::builder().build();
        let mut pna = libpna::Archive::read_header(Cursor::new(bytes)).unwrap();
        let mut entries = pna.entries_with_options(&options);
        for (name, _, kind, content) in cases {
            let entry = entries.next().unwrap().unwrap();
            assert_eq!(entry.name().as_str(), name);
            assert_eq!(entry.data_kind(), kind);
            let metadata = entry.metadata();
            assert_eq!(metadata.owner_uid().unwrap().get(), 123);
            assert_eq!(metadata.owner_gid().unwrap().get(), 456);
            assert_eq!(metadata.permission_mode().unwrap().get(), 0o6754);
            assert_eq!(
                metadata.modified(),
                Some(libpna::Duration::seconds(1_700_000_000))
            );
            assert_eq!(metadata.owner_user_name().unwrap().as_str(), "owner");
            assert_eq!(metadata.owner_group_name().unwrap().as_str(), "group");
            if kind != libpna::DataKind::DIRECTORY {
                let mut actual = String::new();
                entry
                    .reader(&options)
                    .unwrap()
                    .read_to_string(&mut actual)
                    .unwrap();
                assert_eq!(actual, content);
            }
        }
        assert!(entries.next().is_none());
    }

    #[test]
    fn tar_metadata_omits_invalid_owner_names_and_defaults_invalid_ids() {
        let mut header = tar::Header::new_gnu();
        header.set_uid(123);
        header.set_gid(456);
        header.set_mode(0o644);
        header.as_mut_bytes()[108..116].fill(b'x');
        header.as_mut_bytes()[265] = 0xff;
        let metadata = tar_entry_metadata(&header, Path::new("file"), libpna::Duration::seconds(0));
        assert_eq!(metadata.owner_uid().unwrap().get(), 0);
        assert_eq!(metadata.owner_gid().unwrap().get(), 456);
        assert!(metadata.owner_user_name().is_none());
        assert!(metadata.owner_group_name().is_none());
        assert_eq!(metadata.permission_mode().unwrap().get(), 0o644);
    }
}
