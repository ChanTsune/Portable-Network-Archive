use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, HashAlgorithmArgs},
    utils::{self, PathPartExt},
};
use normalize_path::*;
use pna::{
    prelude::*, Archive, EntryBuilder, EntryName, EntryPart, EntryReference, ReadEntry,
    RegularEntry, SolidEntryBuilder, WriteOptions, MIN_CHUNK_BYTES_SIZE, PNA_HEADER,
};
use std::{
    env::temp_dir,
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

impl OwnerOptions {
    #[inline]
    pub(crate) fn new(
        uname: Option<String>,
        gname: Option<String>,
        uid: Option<u32>,
        gid: Option<u32>,
        numeric_owner: bool,
    ) -> Self {
        Self {
            uname: if numeric_owner {
                Some(String::new())
            } else {
                uname
            },
            gname: if numeric_owner {
                Some(String::new())
            } else {
                gname
            },
            uid,
            gid,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct CreateOptions {
    pub(crate) option: WriteOptions,
    pub(crate) keep_options: KeepOptions,
    pub(crate) owner_options: OwnerOptions,
}

pub(crate) fn collect_items<I: IntoIterator<Item = P>, P: Into<PathBuf>>(
    files: I,
    recursive: bool,
    keep_dir: bool,
    gitignore: bool,
    follow_links: bool,
    exclude: Option<Vec<PathBuf>>,
) -> io::Result<Vec<PathBuf>> {
    let mut files = files.into_iter();
    let exclude = exclude.map(|it| it.into_iter().map(|path| path.normalize()));
    let mut target_items = vec![];
    let walker = {
        if let Some(p) = files.next() {
            let mut builder = ignore::WalkBuilder::new(p.into());
            for p in files {
                builder.add(p.into());
            }
            for exclude_path in exclude.into_iter().flatten() {
                builder.add_ignore(exclude_path);
            }
            builder
                .max_depth(if recursive { None } else { Some(0) })
                .hidden(false)
                .ignore(false)
                .git_ignore(gitignore)
                .git_exclude(false)
                .git_global(false)
                .parents(false)
                .follow_links(follow_links)
                .ignore_case_insensitive(false);
            Some(builder.build())
        } else {
            None
        }
    };
    for path in walker.into_iter().flatten() {
        let path = path.map_err(io::Error::other)?.into_path();
        if keep_dir || path.is_file() {
            target_items.push(path);
        }
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
        #[cfg(feature = "memmap")]
        {
            const FILE_SIZE_THRESHOLD: u64 = 50 * 1024 * 1024;
            let meta = fs::metadata(path)?;
            if FILE_SIZE_THRESHOLD < meta.len() {
                let file = utils::mmap::Mmap::open(path)?;
                entry.write_all(&file[..])?;
            } else {
                entry.write_all(&fs::read(path)?)?;
            }
        }
        #[cfg(not(feature = "memmap"))]
        {
            entry.write_all(&fs::read(path)?)?;
        }
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
    hash: HashAlgorithmArgs,
    password: Option<String>,
) -> WriteOptions {
    let (algorithm, level) = compression.algorithm();
    let mut option_builder = WriteOptions::builder();
    option_builder
        .compression(algorithm)
        .compression_level(level.unwrap_or_default())
        .encryption(if password.is_some() {
            cipher.algorithm()
        } else {
            pna::Encryption::No
        })
        .cipher_mode(cipher.mode())
        .hash_algorithm(hash.algorithm())
        .password(password);
    option_builder.build()
}

#[cfg_attr(target_os = "wasi", allow(unused_variables))]
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
            use crate::utils::os::unix::fs::owner::{Group, User};
            use std::os::unix::fs::{MetadataExt, PermissionsExt};

            let mode = meta.permissions().mode() as u16;
            let uid = owner_options.uid.unwrap_or(meta.uid());
            let gid = owner_options.gid.unwrap_or(meta.gid());
            let user = User::from_uid(uid.into()).ok_or(io::ErrorKind::NotFound)?;
            let group = Group::from_gid(gid.into()).ok_or(io::ErrorKind::NotFound)?;
            entry.permission(pna::Permission::new(
                uid.into(),
                owner_options.uname.unwrap_or(user.name().into()),
                gid.into(),
                owner_options.gname.unwrap_or(group.name().into()),
                mode,
            ));
        }
        #[cfg(windows)]
        if keep_options.keep_permission {
            use crate::utils::os::windows::{fs::stat, security::SecurityDescriptor};

            let sd = SecurityDescriptor::try_from(path)?;
            let stat = stat(sd.path.as_ptr() as _)?;
            let mode = stat.st_mode;
            let user = sd.owner_sid()?;
            let group = sd.group_sid()?;
            entry.permission(pna::Permission::new(
                u64::MAX,
                owner_options.uname.unwrap_or(user.name),
                u64::MAX,
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
            let ace_list = utils::acl::get_facl(path)?;
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
            log::warn!("Currently acl is not supported on this platform.");
        }
    }
    #[cfg(not(feature = "acl"))]
    if keep_options.keep_acl {
        log::warn!("Please enable `acl` feature and rebuild and install pna.");
    }
    #[cfg(unix)]
    if keep_options.keep_xattr {
        for attr in utils::os::unix::fs::xattrs::get_xattrs(path)? {
            entry.add_xattr(attr);
        }
    }
    #[cfg(not(unix))]
    if keep_options.keep_xattr {
        log::warn!("Currently extended attribute is not supported on this platform.");
    }
    Ok(entry)
}

pub(crate) fn split_to_parts(
    mut entry_part: EntryPart<&[u8]>,
    first: usize,
    max: usize,
) -> Vec<EntryPart<&[u8]>> {
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
    type Source: Read;
    fn initial_source(&self) -> io::Result<Self::Source>;
    fn next_source(&self, n: usize) -> io::Result<Self::Source>;
}

pub(crate) struct PathArchiveProvider<'p>(&'p Path);

impl<'p> PathArchiveProvider<'p> {
    #[inline]
    pub(crate) const fn new(path: &'p Path) -> Self {
        Self(path)
    }
}

impl<'p> ArchiveProvider for PathArchiveProvider<'p> {
    type Source = fs::File;

    #[inline]
    fn initial_source(&self) -> io::Result<Self::Source> {
        fs::File::open(self.0)
    }

    #[inline]
    fn next_source(&self, n: usize) -> io::Result<Self::Source> {
        fs::File::open(self.0.with_part(n).unwrap())
    }
}

pub(crate) struct StdinArchiveProvider;

impl StdinArchiveProvider {
    #[inline]
    pub(crate) const fn new() -> Self {
        Self
    }
}

impl ArchiveProvider for StdinArchiveProvider {
    type Source = io::StdinLock<'static>;

    #[inline]
    fn initial_source(&self) -> io::Result<Self::Source> {
        Ok(io::stdin().lock())
    }

    #[inline]
    fn next_source(&self, _: usize) -> io::Result<Self::Source> {
        Ok(io::stdin().lock())
    }
}

pub(crate) trait TransformStrategy {
    fn transform<W, T, F>(
        archive: &mut Archive<W>,
        password: Option<&str>,
        read_entry: io::Result<ReadEntry<T>>,
        transformer: F,
    ) -> io::Result<()>
    where
        W: Write,
        T: AsRef<[u8]>,
        F: FnMut(io::Result<RegularEntry<T>>) -> io::Result<Option<RegularEntry<T>>>,
        RegularEntry<T>: From<RegularEntry>,
        RegularEntry<T>: Entry;
}

pub(crate) struct TransformStrategyUnSolid;

impl TransformStrategy for TransformStrategyUnSolid {
    fn transform<W, T, F>(
        archive: &mut Archive<W>,
        password: Option<&str>,
        read_entry: io::Result<ReadEntry<T>>,
        mut transformer: F,
    ) -> io::Result<()>
    where
        W: Write,
        T: AsRef<[u8]>,
        F: FnMut(io::Result<RegularEntry<T>>) -> io::Result<Option<RegularEntry<T>>>,
        RegularEntry<T>: From<RegularEntry>,
        RegularEntry<T>: Entry,
    {
        match read_entry? {
            ReadEntry::Solid(s) => {
                for n in s.entries(password)? {
                    if let Some(entry) = transformer(n.map(Into::into))? {
                        archive.add_entry(entry)?;
                    }
                }
                Ok(())
            }
            ReadEntry::Regular(n) => {
                if let Some(entry) = transformer(Ok(n))? {
                    archive.add_entry(entry)?;
                }
                Ok(())
            }
        }
    }
}

pub(crate) fn run_across_archive<P, F>(provider: P, mut processor: F) -> io::Result<()>
where
    P: ArchiveProvider,
    F: FnMut(&mut Archive<P::Source>) -> io::Result<()>,
{
    let mut archive = Archive::read_header(provider.initial_source()?)?;
    let mut num_archive = 1;
    loop {
        processor(&mut archive)?;
        if archive.has_next_archive() {
            num_archive += 1;
            let next_reader = provider.next_source(num_archive)?;
            archive = archive.read_next_archive(next_reader)?;
        } else {
            break;
        }
    }
    Ok(())
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
    let password = password_provider();
    run_read_entries(archive_provider, |entry| match entry? {
        ReadEntry::Solid(solid) => {
            for s in solid.entries(password)? {
                processor(s)?;
            }
            Ok(())
        }
        ReadEntry::Regular(regular) => processor(Ok(regular)),
    })
}

#[cfg(feature = "memmap")]
pub(crate) fn run_across_archive_mem<P, F>(path: P, processor: F) -> io::Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&mut Archive<&[u8]>) -> io::Result<()>,
{
    fn inner<F>(
        num_archive: usize,
        provider: PathArchiveProvider,
        mut archive: Archive<&[u8]>,
        mut processor: F,
    ) -> io::Result<()>
    where
        F: FnMut(&mut Archive<&[u8]>) -> io::Result<()>,
    {
        processor(&mut archive)?;
        if archive.has_next_archive() {
            let next_reader = provider.next_source(num_archive)?;
            let file = utils::mmap::Mmap::try_from(next_reader)?;
            inner(
                num_archive + 1,
                provider,
                archive.read_next_archive_from_slice(&file[..])?,
                processor,
            )?;
        }
        Ok(())
    }
    let provider = PathArchiveProvider::new(path.as_ref());
    let initial_source = provider.initial_source()?;
    let file = utils::mmap::Mmap::try_from(initial_source)?;
    let archive = Archive::read_header_from_slice(&file[..])?;
    inner(2, provider, archive, processor)
}

#[cfg(feature = "memmap")]
pub(crate) fn run_read_entries_mem<P, F>(path: P, mut processor: F) -> io::Result<()>
where
    P: AsRef<Path>,
    F: FnMut(io::Result<ReadEntry<std::borrow::Cow<[u8]>>>) -> io::Result<()>,
{
    run_across_archive_mem(path, |archive| {
        for entry in archive.entries_slice() {
            processor(entry)?;
        }
        Ok(())
    })
}

#[cfg(feature = "memmap")]
pub(crate) fn run_entries<'p, P, Provider, F>(
    path: P,
    mut password_provider: Provider,
    mut processor: F,
) -> io::Result<()>
where
    P: AsRef<Path>,
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<RegularEntry<std::borrow::Cow<[u8]>>>) -> io::Result<()>,
{
    let password = password_provider();
    run_read_entries_mem(path, |entry| {
        match entry? {
            ReadEntry::Solid(s) => {
                for r in s.entries(password)? {
                    processor(r.map(Into::into))?;
                }
            }
            ReadEntry::Regular(r) => processor(Ok(r))?,
        }
        Ok(())
    })
}

#[cfg(feature = "memmap")]
pub(crate) fn run_manipulate_entry<'p, O, P, Provider, F, Transform>(
    output_path: O,
    input_path: P,
    mut password_provider: Provider,
    mut processor: F,
    _strategy: Transform,
) -> io::Result<()>
where
    O: AsRef<Path>,
    P: AsRef<Path>,
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(
        io::Result<RegularEntry<std::borrow::Cow<[u8]>>>,
    ) -> io::Result<Option<RegularEntry<std::borrow::Cow<[u8]>>>>,
    Transform: TransformStrategy,
{
    let password = password_provider();
    let random = rand::random::<usize>();
    let temp_path = temp_dir().join(format!("{}.pna.tmp", random));
    let outfile = fs::File::create(&temp_path)?;
    let mut out_archive = Archive::write_header(outfile)?;

    run_read_entries_mem(input_path, |entry| {
        Transform::transform(&mut out_archive, password, entry, |it| processor(it))
    })?;

    out_archive.finalize()?;
    utils::fs::mv(temp_path, output_path)?;
    Ok(())
}

pub(crate) fn run_read_entries<F>(
    archive_provider: impl ArchiveProvider,
    mut processor: F,
) -> io::Result<()>
where
    F: FnMut(io::Result<ReadEntry>) -> io::Result<()>,
{
    run_across_archive(archive_provider, |archive| {
        for entry in archive.entries() {
            processor(entry)?;
        }
        Ok(())
    })
}

#[cfg(not(feature = "memmap"))]
pub(crate) fn run_manipulate_entry<'p, O, P, Provider, F, Transform>(
    output_path: O,
    input_path: P,
    mut password_provider: Provider,
    mut processor: F,
    _strategy: Transform,
) -> io::Result<()>
where
    O: AsRef<Path>,
    P: AsRef<Path>,
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<RegularEntry>) -> io::Result<Option<RegularEntry>>,
    Transform: TransformStrategy,
{
    let password = password_provider();
    let output_path = output_path.as_ref();
    let random = rand::random::<usize>();
    let temp_path = temp_dir().join(format!("{}.pna.tmp", random));
    let outfile = fs::File::create(&temp_path)?;
    let mut out_archive = Archive::write_header(outfile)?;

    run_read_entries(PathArchiveProvider(input_path.as_ref()), |entry| {
        Transform::transform(&mut out_archive, password, entry, |it| processor(it))
    })?;

    out_archive.finalize()?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    utils::fs::mv(temp_path, output_path)?;
    Ok(())
}

#[cfg(not(feature = "memmap"))]
pub(crate) fn run_entries<'p, P, Provider, F>(
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
        |n| fs::File::create(get_part_path(archive, n)),
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
        let p = EntryPart::from(entry?);
        let parts = split_to_parts(
            p.as_ref(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn collect_items_only_file() {
        let source = [format!(
            "{}/../resources/test/raw",
            env!("CARGO_MANIFEST_DIR")
        )];
        let items = collect_items(source, false, false, false, false, None).unwrap();
        assert_eq!(
            items.into_iter().collect::<HashSet<_>>(),
            [].into_iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn collect_items_keep_dir() {
        let source = [format!(
            "{}/../resources/test/raw",
            env!("CARGO_MANIFEST_DIR")
        )];
        let items = collect_items(source, false, true, false, false, None).unwrap();
        assert_eq!(
            items.into_iter().collect::<HashSet<_>>(),
            [format!(
                "{}/../resources/test/raw",
                env!("CARGO_MANIFEST_DIR")
            )]
            .into_iter()
            .map(Into::into)
            .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn collect_items_recursive() {
        let source = [format!(
            "{}/../resources/test/raw",
            env!("CARGO_MANIFEST_DIR")
        )];
        let items = collect_items(source, true, false, false, false, None).unwrap();
        assert_eq!(
            items.into_iter().collect::<HashSet<_>>(),
            [
                format!(
                    "{}/../resources/test/raw/first/second/third/pna.txt",
                    env!("CARGO_MANIFEST_DIR")
                ),
                format!(
                    "{}/../resources/test/raw/images/icon.bmp",
                    env!("CARGO_MANIFEST_DIR")
                ),
                format!(
                    "{}/../resources/test/raw/images/icon.png",
                    env!("CARGO_MANIFEST_DIR")
                ),
                format!(
                    "{}/../resources/test/raw/images/icon.svg",
                    env!("CARGO_MANIFEST_DIR")
                ),
                format!(
                    "{}/../resources/test/raw/parent/child.txt",
                    env!("CARGO_MANIFEST_DIR")
                ),
                format!(
                    "{}/../resources/test/raw/pna/empty.pna",
                    env!("CARGO_MANIFEST_DIR")
                ),
                format!(
                    "{}/../resources/test/raw/pna/nest.pna",
                    env!("CARGO_MANIFEST_DIR")
                ),
                format!(
                    "{}/../resources/test/raw/empty.txt",
                    env!("CARGO_MANIFEST_DIR")
                ),
                format!(
                    "{}/../resources/test/raw/text.txt",
                    env!("CARGO_MANIFEST_DIR")
                ),
            ]
            .into_iter()
            .map(Into::into)
            .collect::<HashSet<_>>()
        );
    }
}
