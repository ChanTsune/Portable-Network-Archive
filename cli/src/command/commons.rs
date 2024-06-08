use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs},
    utils::PathPartExt,
};
#[cfg(unix)]
use nix::unistd::{Group, User};
use normalize_path::*;
use pna::{
    Archive, Entry, EntryBuilder, EntryName, EntryPart, EntryReference, RegularEntry, WriteOption,
    MIN_CHUNK_BYTES_SIZE, PNA_HEADER,
};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{
    fs,
    io::{self, prelude::*},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct KeepOptions {
    pub(crate) keep_timestamp: bool,
    pub(crate) keep_permission: bool,
    pub(crate) keep_xattr: bool,
    pub(crate) keep_acl: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct OwnerOptions {
    pub(crate) uname: Option<String>,
    pub(crate) gname: Option<String>,
    pub(crate) uid: Option<u32>,
    pub(crate) gid: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct CreateOptions {
    pub(crate) option: WriteOption,
    pub(crate) keep_options: KeepOptions,
    pub(crate) owner_options: OwnerOptions,
}

pub(crate) fn collect_items<I: IntoIterator<Item = P>, P: Into<PathBuf>>(
    files: I,
    recursive: bool,
    keep_dir: bool,
    exclude: Option<Vec<PathBuf>>,
) -> io::Result<Vec<PathBuf>> {
    let exclude = exclude
        .as_ref()
        .map(|it| it.iter().map(|path| path.normalize()).collect::<Vec<_>>());
    fn inner(
        result: &mut Vec<PathBuf>,
        path: &Path,
        recursive: bool,
        keep_dir: bool,
        exclude: Option<&Vec<PathBuf>>,
    ) -> io::Result<()> {
        let cpath = path.normalize();
        if let Some(exclude) = exclude {
            if exclude.iter().any(|it| it.eq(&cpath)) {
                return Ok(());
            }
        }
        if path.is_dir() {
            if keep_dir {
                result.push(path.to_path_buf());
            }
            if recursive {
                for p in fs::read_dir(path)? {
                    inner(result, &p?.path(), recursive, keep_dir, exclude)?;
                }
            }
        } else if path.is_file() {
            result.push(path.to_path_buf());
        }
        Ok(())
    }
    let mut target_items = vec![];
    for p in files {
        inner(
            &mut target_items,
            &p.into(),
            recursive,
            keep_dir,
            exclude.as_ref(),
        )?;
    }
    Ok(target_items)
}

pub(crate) fn create_entry(
    path: &Path,
    CreateOptions {
        option,
        keep_options,
        owner_options,
    }: CreateOptions,
) -> io::Result<RegularEntry> {
    if path.is_symlink() {
        let source = fs::read_link(path)?;
        let entry = EntryBuilder::new_symbolic_link(
            EntryName::from_lossy(path),
            EntryReference::from_lossy(source.as_path()),
        )?;
        return apply_metadata(entry, path, keep_options, owner_options)?.build();
    } else if path.is_file() {
        let mut entry = EntryBuilder::new_file(EntryName::from_lossy(path), option)?;
        entry.write_all(&fs::read(path)?)?;
        return apply_metadata(entry, path, keep_options, owner_options)?.build();
    } else if path.is_dir() {
        let entry = EntryBuilder::new_dir(EntryName::from_lossy(path));
        return apply_metadata(entry, path, keep_options, owner_options)?.build();
    }
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Currently not a regular file is not supported.",
    ))
}

pub(crate) fn entry_option(
    compression: CompressionAlgorithmArgs,
    cipher: CipherAlgorithmArgs,
    password: Option<String>,
) -> WriteOption {
    let (algorithm, level) = compression.algorithm();
    let mut option_builder = WriteOption::builder();
    option_builder
        .compression(algorithm)
        .compression_level(level.unwrap_or_default())
        .encryption(if password.is_some() {
            cipher.algorithm()
        } else {
            pna::Encryption::No
        })
        .cipher_mode(cipher.mode())
        .password(password);
    option_builder.build()
}

pub(crate) fn apply_metadata(
    mut entry: EntryBuilder,
    path: &Path,
    keep_options: KeepOptions,
    owner_options: OwnerOptions,
) -> io::Result<EntryBuilder> {
    if keep_options.keep_timestamp || keep_options.keep_permission {
        let meta = fs::metadata(path)?;
        if keep_options.keep_timestamp {
            if let Ok(c) = meta.created() {
                if let Ok(created_since_unix_epoch) = c.duration_since(UNIX_EPOCH) {
                    entry.created(created_since_unix_epoch);
                }
            }
            if let Ok(m) = meta.modified() {
                if let Ok(modified_since_unix_epoch) = m.duration_since(UNIX_EPOCH) {
                    entry.modified(modified_since_unix_epoch);
                }
            }
            if let Ok(a) = meta.accessed() {
                if let Ok(accessed_since_unix_epoch) = a.duration_since(UNIX_EPOCH) {
                    entry.accessed(accessed_since_unix_epoch);
                }
            }
        }
        #[cfg(unix)]
        if keep_options.keep_permission {
            let mode = meta.permissions().mode() as u16;
            let uid = owner_options.uid.unwrap_or(meta.uid());
            let gid = owner_options.gid.unwrap_or(meta.gid());
            let user = User::from_uid(uid.into())?.unwrap();
            let group = Group::from_gid(gid.into())?.unwrap();
            entry.permission(pna::Permission::new(
                uid.into(),
                owner_options.uname.unwrap_or(user.name),
                gid.into(),
                owner_options.gname.unwrap_or(group.name),
                mode,
            ));
        }
    }
    #[cfg(feature = "acl")]
    {
        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "macos",
            windows
        ))]
        if keep_options.keep_acl {
            use crate::chunk;
            use pna::RawChunk;
            let ace_list = crate::utils::acl::get_facl(path)?;
            for ace in ace_list {
                entry.add_extra_chunk(RawChunk::from_data(chunk::faCe, ace.to_bytes()));
            }
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "macos",
            windows
        )))]
        if keep_options.keep_acl {
            eprintln!("Currently acl is not supported on this platform.");
        }
    }
    #[cfg(not(feature = "acl"))]
    if keep_options.keep_acl {
        eprintln!("Please enable `acl` feature and rebuild and install pna.");
    }
    #[cfg(unix)]
    if keep_options.keep_xattr {
        if xattr::SUPPORTED_PLATFORM {
            let xattrs = xattr::list(path)?;
            for name in xattrs {
                let value = xattr::get(path, &name)?.unwrap_or_default();
                entry.add_xattr(pna::ExtendedAttribute::new(
                    name.to_string_lossy().into(),
                    value,
                ));
            }
        } else {
            eprintln!("Currently extended attribute is not supported on this platform.");
        }
    }
    #[cfg(not(unix))]
    if keep_options.keep_xattr {
        eprintln!("Currently extended attribute is not supported on this platform.");
    }
    Ok(entry)
}

pub(crate) fn split_to_parts(
    mut entry_part: EntryPart,
    first: usize,
    max: usize,
) -> Vec<EntryPart> {
    let mut parts = vec![];
    let mut split_size = first;
    loop {
        match entry_part.split(split_size) {
            (write_part, Some(remaining_part)) => {
                parts.push(write_part);
                entry_part = remaining_part;
                split_size = max;
            }
            (write_part, None) => {
                parts.push(write_part);
                break;
            }
        }
    }
    parts
}

pub(crate) trait ArchiveProvider {
    type A: Read;
    fn initial_archive(&self) -> io::Result<Self::A>;
    fn next_archive(&self, n: usize) -> io::Result<Self::A>;
}

pub(crate) struct PathArchiveProvider<'p>(&'p Path);

impl<'p> PathArchiveProvider<'p> {
    pub(crate) fn new(path: &'p Path) -> Self {
        Self(path)
    }
}

impl<'p> ArchiveProvider for PathArchiveProvider<'p> {
    type A = fs::File;

    fn initial_archive(&self) -> io::Result<Self::A> {
        fs::File::open(self.0)
    }

    fn next_archive(&self, n: usize) -> io::Result<Self::A> {
        fs::File::open(self.0.with_part(n).unwrap())
    }
}

pub(crate) struct StdinArchiveProvider;

impl StdinArchiveProvider {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl ArchiveProvider for StdinArchiveProvider {
    type A = io::StdinLock<'static>;

    fn initial_archive(&self) -> io::Result<Self::A> {
        Ok(io::stdin().lock())
    }

    fn next_archive(&self, _: usize) -> io::Result<Self::A> {
        Ok(io::stdin().lock())
    }
}

pub(crate) fn run_across_archive<P, F>(provider: P, mut processor: F) -> io::Result<()>
where
    P: ArchiveProvider,
    F: FnMut(&mut Archive<P::A>) -> io::Result<()>,
{
    let mut archive = Archive::read_header(provider.initial_archive()?)?;
    let mut num_archive = 1;
    loop {
        processor(&mut archive)?;
        if archive.next_archive() {
            num_archive += 1;
            let next_reader = provider.next_archive(num_archive)?;
            archive = archive.read_next_archive(next_reader)?;
        } else {
            break;
        }
    }
    Ok(())
}

pub(crate) fn run_across_archive_path<R, F>(path: R, processor: F) -> io::Result<()>
where
    R: AsRef<Path>,
    F: FnMut(&mut Archive<fs::File>) -> io::Result<()>,
{
    let path = path.as_ref();
    let file = PathArchiveProvider::new(path);
    run_across_archive(file, processor)
}

pub(crate) fn run_process_archive<'p, Provider, F>(
    archive_provider: impl ArchiveProvider,
    mut password_provider: Provider,
    mut processor: F,
) -> io::Result<()>
where
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<RegularEntry>) -> io::Result<()>,
{
    run_across_archive(archive_provider, |archive| {
        for entry in archive.entries_with_password(password_provider()) {
            processor(entry)?;
        }
        Ok(())
    })
}

pub(crate) fn run_process_archive_path<'p, P, Provider, F>(
    path: P,
    password_provider: Provider,
    processor: F,
) -> io::Result<()>
where
    P: AsRef<Path>,
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<RegularEntry>) -> io::Result<()>,
{
    let path = path.as_ref();
    let provider = PathArchiveProvider(path);
    run_process_archive(provider, password_provider, processor)
}

pub(crate) fn write_split_archive(
    archive: impl AsRef<Path>,
    entries: impl Iterator<Item = io::Result<impl Entry + Sized>>,
    max_file_size: usize,
) -> io::Result<()> {
    write_split_archive_path(
        archive,
        entries,
        |base, n| base.with_part(n).unwrap(),
        max_file_size,
    )
}

pub(crate) fn write_split_archive_path<F, P>(
    archive: impl AsRef<Path>,
    entries: impl Iterator<Item = io::Result<impl Entry + Sized>>,
    mut get_part_path: F,
    max_file_size: usize,
) -> io::Result<()>
where
    F: FnMut(&Path, usize) -> P,
    P: AsRef<Path>,
{
    let archive = archive.as_ref();
    let first_item_path = get_part_path(archive, 1);
    let first_item_path = first_item_path.as_ref();
    let file = fs::File::create(first_item_path)?;
    write_split_archive_writer(
        file,
        entries,
        |n| {
            let part_n_name = get_part_path(archive, n);
            fs::File::create(&part_n_name)
        },
        max_file_size,
        |n| {
            if n == 1 {
                fs::rename(first_item_path, archive)?;
            };
            Ok(())
        },
    )
}

pub(crate) fn write_split_archive_writer<W, F, C>(
    initial_writer: W,
    entries: impl Iterator<Item = io::Result<impl Entry + Sized>>,
    mut get_next_writer: F,
    max_file_size: usize,
    mut on_complete: C,
) -> io::Result<()>
where
    W: Write,
    F: FnMut(usize) -> io::Result<W>,
    C: FnMut(usize) -> io::Result<()>,
{
    let mut part_num = 1;
    let mut writer = Archive::write_header(initial_writer)?;

    // NOTE: max_file_size - (PNA_HEADER + AHED + ANXT + AEND)
    let max_file_size = max_file_size - (PNA_HEADER.len() + MIN_CHUNK_BYTES_SIZE * 3 + 8);
    let mut written_entry_size = 0;
    for entry in entries {
        let parts = split_to_parts(
            EntryPart::from(entry?),
            max_file_size - written_entry_size,
            max_file_size,
        );
        for part in parts {
            if written_entry_size + part.bytes_len() > max_file_size {
                part_num += 1;
                let file = get_next_writer(part_num)?;
                writer = writer.split_to_next_archive(file)?;
                written_entry_size = 0;
            }
            written_entry_size += writer.add_entry_part(part)?;
        }
    }
    writer.finalize()?;
    on_complete(part_num)?;
    Ok(())
}
