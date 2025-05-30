use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, HashAlgorithmArgs},
    utils::{
        self,
        env::temp_dir_or_else,
        re::{
            bsd::{SubstitutionRule, SubstitutionRules},
            gnu::{TransformRule, TransformRules},
        },
        BsdGlobPatterns, PathPartExt,
    },
};
use path_slash::*;
use pna::{
    prelude::*, Archive, EntryBuilder, EntryName, EntryPart, EntryReference, NormalEntry,
    ReadEntry, SolidEntryBuilder, WriteOptions, MIN_CHUNK_BYTES_SIZE, PNA_HEADER,
};
use std::{
    borrow::Cow,
    fs,
    io::{self, prelude::*},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
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
    pub(crate) time_options: TimeOptions,
    pub(crate) follow_links: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum PathTransformers {
    BsdSubstitutions(SubstitutionRules),
    GnuTransforms(TransformRules),
}

impl PathTransformers {
    pub(crate) fn new(
        substitutions: Option<Vec<SubstitutionRule>>,
        transforms: Option<Vec<TransformRule>>,
    ) -> Option<Self> {
        if let Some(s) = substitutions {
            Some(Self::BsdSubstitutions(SubstitutionRules::new(s)))
        } else {
            transforms.map(|t| Self::GnuTransforms(TransformRules::new(t)))
        }
    }
    #[inline]
    pub(crate) fn apply(
        &self,
        input: impl Into<String>,
        is_symlink: bool,
        is_hardlink: bool,
    ) -> String {
        match self {
            Self::BsdSubstitutions(s) => s.apply(input, is_symlink, is_hardlink),
            Self::GnuTransforms(t) => t.apply(input, is_symlink, is_hardlink),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct TimeOptions {
    pub(crate) mtime: Option<SystemTime>,
    pub(crate) clamp_mtime: bool,
    pub(crate) ctime: Option<SystemTime>,
    pub(crate) clamp_ctime: bool,
    pub(crate) atime: Option<SystemTime>,
    pub(crate) clamp_atime: bool,
}

pub(crate) fn collect_items(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    recursive: bool,
    keep_dir: bool,
    gitignore: bool,
    follow_links: bool,
    exclude: Exclude,
) -> io::Result<Vec<PathBuf>> {
    let mut files = files.into_iter();
    if let Some(p) = files.next() {
        let mut builder = ignore::WalkBuilder::new(p);
        for p in files {
            builder.add(p);
        }
        builder.filter_entry(move |e| !exclude.excluded(e.path().to_slash_lossy()));
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
        let walker = builder.build();
        walker
            .filter_map(|path| match path {
                Ok(path) => {
                    let path = path.into_path();
                    (keep_dir || path.is_file()).then_some(Ok(path))
                }
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(io::Error::other)
    } else {
        Ok(Vec::new())
    }
}

pub(crate) fn collect_split_archives(first: impl AsRef<Path>) -> io::Result<Vec<fs::File>> {
    let mut archives = Vec::new();
    let mut n = 1;
    let mut target_archive = Cow::from(first.as_ref());
    while fs::exists(&target_archive)? {
        archives.push(fs::File::open(&target_archive)?);
        n += 1;
        target_archive = target_archive.with_part(n).expect("").into();
    }
    Ok(archives)
}

pub(crate) fn create_entry(
    path: &Path,
    CreateOptions {
        option,
        keep_options,
        owner_options,
        time_options,
        follow_links,
    }: &CreateOptions,
    substitutions: &Option<PathTransformers>,
) -> io::Result<NormalEntry> {
    let entry_name = if let Some(substitutions) = substitutions {
        EntryName::from(substitutions.apply(path.to_string_lossy(), false, false))
    } else {
        EntryName::from_lossy(path)
    };
    if !follow_links && path.is_symlink() {
        let source = fs::read_link(path)?;
        let reference = if let Some(substitutions) = substitutions {
            EntryReference::from(substitutions.apply(path.to_string_lossy(), true, false))
        } else {
            EntryReference::from_lossy(source)
        };
        let entry = EntryBuilder::new_symbolic_link(entry_name, reference)?;
        return apply_metadata(
            entry,
            path,
            keep_options,
            owner_options,
            time_options,
            fs::symlink_metadata,
        )?
        .build();
    } else if path.is_file() {
        let mut entry = EntryBuilder::new_file(entry_name, option)?;
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
        return apply_metadata(
            entry,
            path,
            keep_options,
            owner_options,
            time_options,
            fs::metadata,
        )?
        .build();
    } else if path.is_dir() {
        let entry = EntryBuilder::new_dir(entry_name);
        return apply_metadata(
            entry,
            path,
            keep_options,
            owner_options,
            time_options,
            fs::metadata,
        )?
        .build();
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
    password: Option<&str>,
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
pub(crate) fn apply_metadata<'p>(
    mut entry: EntryBuilder,
    path: &'p Path,
    keep_options: &KeepOptions,
    owner_options: &OwnerOptions,
    time_options: &TimeOptions,
    metadata: impl Fn(&'p Path) -> io::Result<fs::Metadata>,
) -> io::Result<EntryBuilder> {
    if keep_options.keep_timestamp || keep_options.keep_permission {
        let meta = metadata(path)?;
        if keep_options.keep_timestamp {
            let ctime = clamped_time(
                meta.created().ok(),
                time_options.ctime,
                time_options.clamp_ctime,
            );
            if let Some(c) = ctime {
                if let Ok(created_since_unix_epoch) = c.duration_since(UNIX_EPOCH) {
                    entry.created(created_since_unix_epoch);
                }
            }
            let mtime = clamped_time(
                meta.modified().ok(),
                time_options.mtime,
                time_options.clamp_mtime,
            );
            if let Some(m) = mtime {
                if let Ok(modified_since_unix_epoch) = m.duration_since(UNIX_EPOCH) {
                    entry.modified(modified_since_unix_epoch);
                }
            }
            let atime = clamped_time(
                meta.accessed().ok(),
                time_options.atime,
                time_options.clamp_atime,
            );
            if let Some(a) = atime {
                if let Ok(accessed_since_unix_epoch) = a.duration_since(UNIX_EPOCH) {
                    entry.accessed(accessed_since_unix_epoch);
                }
            }
        }
        #[cfg(unix)]
        if keep_options.keep_permission {
            use crate::utils::fs::{Group, User};
            use std::os::unix::fs::{MetadataExt, PermissionsExt};

            let mode = meta.permissions().mode() as u16;
            let uid = owner_options.uid.unwrap_or(meta.uid());
            let gid = owner_options.gid.unwrap_or(meta.gid());
            entry.permission(pna::Permission::new(
                uid.into(),
                match owner_options.uname.as_deref() {
                    None => User::from_uid(uid.into())?
                        .name()
                        .unwrap_or_default()
                        .into(),
                    Some(uname) => uname.into(),
                },
                gid.into(),
                match owner_options.gname.as_deref() {
                    None => Group::from_gid(gid.into())?
                        .name()
                        .unwrap_or_default()
                        .into(),
                    Some(gname) => gname.into(),
                },
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
                owner_options.uid.map_or(u64::MAX, Into::into),
                owner_options.uname.clone().unwrap_or(user.name),
                owner_options.gid.map_or(u64::MAX, Into::into),
                owner_options.gname.clone().unwrap_or(group.name),
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
            let acl = utils::acl::get_facl(path)?;
            entry.add_extra_chunk(RawChunk::from_data(chunk::faCl, acl.platform.to_bytes()));
            for ace in acl.entries {
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

fn clamped_time(
    fs_time: Option<SystemTime>,
    specified_time: Option<SystemTime>,
    clamp: bool,
) -> Option<SystemTime> {
    if let Some(specified_time) = specified_time {
        if clamp {
            if let Some(fs_time) = fs_time {
                if fs_time < specified_time {
                    Some(fs_time)
                } else {
                    Some(specified_time)
                }
            } else {
                Some(specified_time)
            }
        } else {
            Some(specified_time)
        }
    } else {
        fs_time
    }
}

pub(crate) fn split_to_parts(
    mut entry_part: EntryPart<&[u8]>,
    first: usize,
    max: usize,
) -> io::Result<Vec<EntryPart<&[u8]>>> {
    let mut parts = vec![];
    let mut split_size = first;
    loop {
        match entry_part.try_split(split_size) {
            Ok((write_part, Some(remaining_part))) => {
                parts.push(write_part);
                entry_part = remaining_part;
                split_size = max;
            }
            Ok((write_part, None)) => {
                parts.push(write_part);
                break;
            }
            Err(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("A chunk was detected that could not be divided into chunks smaller than the given size {max}")
            ))
        }
    }
    Ok(parts)
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
        F: FnMut(io::Result<NormalEntry<T>>) -> io::Result<Option<NormalEntry<T>>>,
        NormalEntry<T>: From<NormalEntry>,
        NormalEntry<T>: Entry;
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
        F: FnMut(io::Result<NormalEntry<T>>) -> io::Result<Option<NormalEntry<T>>>,
        NormalEntry<T>: From<NormalEntry>,
        NormalEntry<T>: Entry,
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
            ReadEntry::Normal(n) => {
                if let Some(entry) = transformer(Ok(n))? {
                    archive.add_entry(entry)?;
                }
                Ok(())
            }
        }
    }
}

pub(crate) struct TransformStrategyKeepSolid;

impl TransformStrategy for TransformStrategyKeepSolid {
    fn transform<W, T, F>(
        archive: &mut Archive<W>,
        password: Option<&str>,
        read_entry: io::Result<ReadEntry<T>>,
        mut transformer: F,
    ) -> io::Result<()>
    where
        W: Write,
        T: AsRef<[u8]>,
        F: FnMut(io::Result<NormalEntry<T>>) -> io::Result<Option<NormalEntry<T>>>,
        NormalEntry<T>: From<NormalEntry>,
        NormalEntry<T>: Entry,
    {
        match read_entry? {
            ReadEntry::Solid(s) => {
                let header = s.header();
                let mut builder = SolidEntryBuilder::new(
                    WriteOptions::builder()
                        .compression(header.compression())
                        .encryption(header.encryption())
                        .cipher_mode(header.cipher_mode())
                        .password(password)
                        .build(),
                )?;
                for n in s.entries(password)? {
                    if let Some(entry) = transformer(n.map(Into::into))? {
                        builder.add_entry(entry)?;
                    }
                }
                archive.add_entry(builder.build()?)?;
                Ok(())
            }
            ReadEntry::Normal(n) => {
                if let Some(entry) = transformer(Ok(n))? {
                    archive.add_entry(entry)?;
                }
                Ok(())
            }
        }
    }
}

// TODO:
// pub(crate) struct TransformStrategyToSolid;

pub(crate) fn run_across_archive<R, F>(
    provider: impl IntoIterator<Item = R>,
    mut processor: F,
) -> io::Result<()>
where
    R: Read,
    F: FnMut(&mut Archive<R>) -> io::Result<()>,
{
    let mut iter = provider.into_iter();
    let mut archive = Archive::read_header(iter.next().expect(""))?;
    loop {
        processor(&mut archive)?;
        if archive.has_next_archive() {
            let next_reader = iter.next().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Archive is split, but no subsequent archives are found",
                )
            })?;
            archive = archive.read_next_archive(next_reader)?;
        } else {
            break;
        }
    }
    Ok(())
}

pub(crate) fn run_process_archive<'p, Provider, F>(
    archive_provider: impl IntoIterator<Item = impl Read>,
    mut password_provider: Provider,
    mut processor: F,
) -> io::Result<()>
where
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<NormalEntry>) -> io::Result<()>,
{
    let password = password_provider();
    run_read_entries(archive_provider, |entry| match entry? {
        ReadEntry::Solid(solid) => solid.entries(password)?.try_for_each(&mut processor),
        ReadEntry::Normal(regular) => processor(Ok(regular)),
    })
}

#[cfg(feature = "memmap")]
pub(crate) fn run_across_archive_mem<F>(archives: Vec<fs::File>, mut processor: F) -> io::Result<()>
where
    F: FnMut(&mut Archive<&[u8]>) -> io::Result<()>,
{
    let archives = archives
        .into_iter()
        .map(utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;

    let mut idx = 0;
    let mut archive = Archive::read_header_from_slice(&archives[idx])?;

    loop {
        processor(&mut archive)?;
        if !archive.has_next_archive() {
            break;
        }
        idx += 1;
        if idx >= archives.len() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Archive is split, but no subsequent archives are found",
            ));
        }
        archive = archive.read_next_archive_from_slice(&archives[idx][..])?;
    }
    Ok(())
}

#[cfg(feature = "memmap")]
pub(crate) fn run_read_entries_mem<F>(archives: Vec<fs::File>, mut processor: F) -> io::Result<()>
where
    F: FnMut(io::Result<ReadEntry<std::borrow::Cow<[u8]>>>) -> io::Result<()>,
{
    run_across_archive_mem(archives, |archive| {
        archive.entries_slice().try_for_each(&mut processor)
    })
}

#[cfg(feature = "memmap")]
pub(crate) fn run_entries<'p, Provider, F>(
    archives: Vec<fs::File>,
    mut password_provider: Provider,
    mut processor: F,
) -> io::Result<()>
where
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<NormalEntry<std::borrow::Cow<[u8]>>>) -> io::Result<()>,
{
    let password = password_provider();
    run_read_entries_mem(archives, |entry| match entry? {
        ReadEntry::Solid(s) => s
            .entries(password)?
            .try_for_each(|r| processor(r.map(Into::into))),
        ReadEntry::Normal(r) => processor(Ok(r)),
    })
}

#[cfg(feature = "memmap")]
pub(crate) fn run_transform_entry<'p, O, Provider, F, Transform>(
    output_path: O,
    archives: Vec<fs::File>,
    mut password_provider: Provider,
    mut processor: F,
    _strategy: Transform,
) -> io::Result<()>
where
    O: AsRef<Path>,
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(
        io::Result<NormalEntry<std::borrow::Cow<[u8]>>>,
    ) -> io::Result<Option<NormalEntry<std::borrow::Cow<[u8]>>>>,
    Transform: TransformStrategy,
{
    let password = password_provider();
    let output_path = output_path.as_ref();
    let random = rand::random::<usize>();
    let temp_dir_path = temp_dir_or_else(|| output_path.parent().unwrap_or_else(|| ".".as_ref()));
    fs::create_dir_all(&temp_dir_path)?;
    let temp_path = temp_dir_path.join(format!("{}.pna.tmp", random));
    let outfile = fs::File::create(&temp_path)?;
    let mut out_archive = Archive::write_header(outfile)?;

    run_read_entries_mem(archives, |entry| {
        Transform::transform(&mut out_archive, password, entry, &mut processor)
    })?;

    out_archive.finalize()?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    utils::fs::mv(temp_path, output_path)?;
    Ok(())
}

pub(crate) fn run_read_entries<F>(
    archive_provider: impl IntoIterator<Item = impl Read>,
    mut processor: F,
) -> io::Result<()>
where
    F: FnMut(io::Result<ReadEntry>) -> io::Result<()>,
{
    run_across_archive(archive_provider, |archive| {
        archive.entries().try_for_each(&mut processor)
    })
}

#[cfg(not(feature = "memmap"))]
pub(crate) fn run_transform_entry<'p, O, Provider, F, Transform>(
    output_path: O,
    archives: impl IntoIterator<Item = impl Read>,
    mut password_provider: Provider,
    mut processor: F,
    _strategy: Transform,
) -> io::Result<()>
where
    O: AsRef<Path>,
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<NormalEntry>) -> io::Result<Option<NormalEntry>>,
    Transform: TransformStrategy,
{
    let password = password_provider();
    let output_path = output_path.as_ref();
    let random = rand::random::<usize>();
    let temp_dir_path = temp_dir_or_else(|| output_path.parent().unwrap_or_else(|| ".".as_ref()));
    fs::create_dir_all(&temp_dir_path)?;
    let temp_path = temp_dir_path.join(format!("{random}.pna.tmp"));
    let outfile = fs::File::create(&temp_path)?;
    let mut out_archive = Archive::write_header(outfile)?;

    run_read_entries(archives, |entry| {
        Transform::transform(&mut out_archive, password, entry, &mut processor)
    })?;

    out_archive.finalize()?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    utils::fs::mv(temp_path, output_path)?;
    Ok(())
}

#[cfg(not(feature = "memmap"))]
pub(crate) fn run_entries<'p, Provider, F>(
    archives: Vec<fs::File>,
    password_provider: Provider,
    processor: F,
) -> io::Result<()>
where
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<NormalEntry>) -> io::Result<()>,
{
    run_process_archive(archives, password_provider, processor)
}

pub(crate) fn write_split_archive(
    archive: impl AsRef<Path>,
    entries: impl Iterator<Item = io::Result<impl Entry + Sized>>,
    max_file_size: usize,
    overwrite: bool,
) -> io::Result<()> {
    write_split_archive_path(
        archive,
        entries,
        |base, n| base.with_part(n).unwrap(),
        max_file_size,
        overwrite,
    )
}

pub(crate) fn write_split_archive_path<F, P>(
    archive: impl AsRef<Path>,
    entries: impl Iterator<Item = io::Result<impl Entry + Sized>>,
    mut get_part_path: F,
    max_file_size: usize,
    overwrite: bool,
) -> io::Result<()>
where
    F: FnMut(&Path, usize) -> P,
    P: AsRef<Path>,
{
    let archive = archive.as_ref();
    let first_item_path = get_part_path(archive, 1);
    let first_item_path = first_item_path.as_ref();
    let file = utils::fs::file_create(first_item_path, overwrite)?;
    write_split_archive_writer(
        file,
        entries,
        |n| utils::fs::file_create(get_part_path(archive, n), overwrite),
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
        )?;
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

#[derive(Clone, Debug)]
pub(crate) struct Exclude {
    pub(crate) include: BsdGlobPatterns,
    pub(crate) exclude: BsdGlobPatterns,
}

impl Exclude {
    #[inline]
    pub(crate) fn excluded(&self, s: impl AsRef<str>) -> bool {
        let s = s.as_ref();
        !self.include.matches_inclusion(s) && self.exclude.matches_exclusion(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::time::Duration;

    fn empty_exclude() -> Exclude {
        Exclude {
            include: Vec::<&str>::new().into(),
            exclude: Vec::<&str>::new().into(),
        }
    }

    #[test]
    fn exclude_empty() {
        let exclude = Exclude {
            include: Vec::<&str>::new().into(),
            exclude: Vec::<&str>::new().into(),
        };
        assert!(!exclude.excluded("a/b/c"));
    }

    #[test]
    fn exclude_exclude() {
        let exclude = Exclude {
            include: Vec::<&str>::new().into(),
            exclude: vec!["a/*"].into(),
        };
        assert!(exclude.excluded("a/b/c"));
    }

    #[test]
    fn exclude_include() {
        let exclude = Exclude {
            include: vec!["a/*/c"].into(),
            exclude: vec!["a/*"].into(),
        };
        assert!(!exclude.excluded("a/b/c"));

        let exclude = Exclude {
            include: vec!["a/*/c"].into(),
            exclude: vec!["a/*/c"].into(),
        };
        assert!(!exclude.excluded("a/b/c"));
    }

    #[test]
    fn collect_items_only_file() {
        let source = [concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../resources/test/raw",
        )];
        let items = collect_items(source, false, false, false, false, empty_exclude()).unwrap();
        assert_eq!(items.into_iter().collect::<HashSet<_>>(), HashSet::new());
    }

    #[test]
    fn collect_items_keep_dir() {
        let source = [concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../resources/test/raw",
        )];
        let items = collect_items(source, false, true, false, false, empty_exclude()).unwrap();
        assert_eq!(
            items.into_iter().collect::<HashSet<_>>(),
            [concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../resources/test/raw",
            )]
            .into_iter()
            .map(Into::into)
            .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn collect_items_recursive() {
        let source = [concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../resources/test/raw",
        )];
        let items = collect_items(source, true, false, false, false, empty_exclude()).unwrap();
        assert_eq!(
            items.into_iter().collect::<HashSet<_>>(),
            [
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/first/second/third/pna.txt"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/images/icon.bmp"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/images/icon.png"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/images/icon.svg"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/parent/child.txt"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/pna/empty.pna"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/pna/nest.pna"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/empty.txt"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/text.txt"
                ),
            ]
            .into_iter()
            .map(Into::into)
            .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn time_use_fs() {
        let result = clamped_time(
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            None,
            false,
        );
        assert_eq!(
            result,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
        );
    }

    #[test]
    fn time_use_specified() {
        let result = clamped_time(
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(2)),
            false,
        );
        assert_eq!(
            result,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(2))
        );
    }

    #[test]
    fn time_use_specified_clamp() {
        let result = clamped_time(
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            Some(SystemTime::UNIX_EPOCH),
            true,
        );
        assert_eq!(result, Some(SystemTime::UNIX_EPOCH));
    }

    #[test]
    fn time_use_specified_no_clamp() {
        let result = clamped_time(
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(2)),
            true,
        );
        assert_eq!(
            result,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
        );
    }
}
