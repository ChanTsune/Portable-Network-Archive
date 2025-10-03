#[cfg(feature = "memmap")]
use crate::command::commons::run_entries_with_options;
#[cfg(any(unix, windows))]
use crate::utils::fs::chown;
use crate::{
    cli::{FileArgsCompat, PasswordArgs},
    command::{
        ask_password,
        commons::{
            collect_split_archives, read_paths, run_process_archive_with_options, Exclude,
            KeepOptions, OwnerOptions, PathTransformers,
        },
        Command,
    },
    utils::{
        self,
        fmt::DurationDisplay,
        fs::{Group, User},
        re::{bsd::SubstitutionRule, gnu::TransformRule},
        GlobPatterns, VCS_FILES,
    },
};
use clap::{ArgGroup, Parser, ValueHint};
#[cfg(unix)]
use nix::unistd::geteuid;
#[cfg(unix)]
use std::os::unix::prelude::UidExt;
use pna::{prelude::*, DataKind, EntryReference, NormalEntry, Permission, ReadOptions};
use std::io::Read;
#[cfg(target_os = "macos")]
use std::os::macos::fs::FileTimesExt;
#[cfg(windows)]
use std::os::windows::fs::FileTimesExt;
use std::{
    borrow::Cow,
    env, fs, io,
    path::{Component, PathBuf},
    time::{Instant, SystemTime},
};

#[derive(Parser, Clone, Debug)]
#[command(
    group(ArgGroup::new("include-group").args(["include"])),
    group(ArgGroup::new("extract-exclude-group").args(["exclude"])),
    group(ArgGroup::new("exclude-from-group").args(["exclude_from"])),
    group(ArgGroup::new("null-requires").arg("null").requires("exclude_from")),
    group(ArgGroup::new("unstable-acl").args(["keep_acl", "no_keep_acl"]).requires("unstable")),
    group(ArgGroup::new("substitution-group").args(["substitutions"])),
    group(ArgGroup::new("transform-group").args(["transforms"])),
    group(ArgGroup::new("path-transform").args(["substitutions", "transforms"])),
    group(ArgGroup::new("owner-flag").args(["same_owner", "no_same_owner"])),
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
    group(ArgGroup::new("exclude-vcs-group").args(["exclude_vcs"])),
)]
#[cfg_attr(windows, command(
    group(ArgGroup::new("windows-unstable-keep-permission").args(["keep_permission"])),
))]
pub(crate) struct ExtractCommand {
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Output directory of extracted files", value_hint = ValueHint::DirPath)]
    pub(crate) out_dir: Option<PathBuf>,
    #[arg(long, help = "Do not overwrite existing files when extracting")]
    pub(crate) keep_old_files: bool,
    #[arg(long, help = "Keep newer files on disk (skip older archive entries)")]
    pub(crate) keep_newer_files: bool,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[arg(
        long,
        visible_alias = "preserve-timestamps",
        help = "Restore the timestamp of the files"
    )]
    pub(crate) keep_timestamp: bool,
    #[arg(
        short = 'm',
        long = "modification-time",
        help = "Do not retain archived modification times; set extracted files to the current time",
        conflicts_with = "keep_timestamp"
    )]
    pub(crate) modification_time: bool,
    #[arg(
        short = 'p',
        long = "keep-permission",
        visible_alias = "preserve-permissions",
        help = "Preserve file permissions (bsdtar -p equivalent)."
    )]
    pub(crate) keep_permission: bool,
    #[arg(
        long = "no-same-permissions",
        help = "Do not retain stored permissions when extracting (bsdtar --no-same-permissions equivalent)",
        conflicts_with = "keep_permission"
    )]
    pub(crate) no_same_permissions: bool,
    #[arg(
        long = "xattrs",
        visible_aliases = ["keep-xattr", "preserve-xattrs"],
        help = "Restore extended attributes (bsdtar --xattrs equivalent)"
    )]
    pub(crate) keep_xattr: bool,
    #[arg(
        long = "no-xattrs",
        help = "Do not restore extended attributes (bsdtar --no-xattrs equivalent)",
        conflicts_with = "keep_xattr"
    )]
    pub(crate) no_keep_xattr: bool,
    #[arg(
        long = "acls",
        visible_aliases = ["keep-acl", "preserve-acls"],
        help = "Restore ACLs (bsdtar --acls equivalent)"
    )]
    pub(crate) keep_acl: bool,
    #[arg(
        long = "no-acls",
        help = "Do not restore ACLs (bsdtar --no-acls equivalent)",
        conflicts_with = "keep_acl"
    )]
    pub(crate) no_keep_acl: bool,
    #[arg(long, help = "Restore user from given name")]
    pub(crate) uname: Option<String>,
    #[arg(long, help = "Restore group from given name")]
    pub(crate) gname: Option<String>,
    #[arg(
        long,
        help = "Overrides the user id in the archive; the user name in the archive will be ignored"
    )]
    pub(crate) uid: Option<u32>,
    #[arg(
        long,
        help = "Overrides the group id in the archive; the group name in the archive will be ignored"
    )]
    pub(crate) gid: Option<u32>,
    #[arg(
        long,
        help = "This is equivalent to --uname \"\" --gname \"\". It causes user and group names in the archive to be ignored in favor of the numeric user and group ids."
    )]
    pub(crate) numeric_owner: bool,
    #[arg(
        long,
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions"
    )]
    include: Option<Vec<String>>,
    #[arg(
        long,
        help = "Exclude path glob",
        value_hint = ValueHint::AnyPath
    )]
    exclude: Option<Vec<String>>,
    #[arg(
        short = 'X',
        long,
        help = "Read exclude patterns from given path",
        value_hint = ValueHint::FilePath
    )]
    exclude_from: Option<PathBuf>,
    #[arg(long, help = "Exclude vcs files")]
    exclude_vcs: bool,
    #[arg(
        long,
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
    #[arg(
        long = "ignore-zeros",
        visible_alias = "ignore-zeroes",
        help = "Continue processing after zero-filled padding blocks"
    )]
    ignore_zeros: bool,
    #[arg(
        long,
        help = "Remove the specified number of leading path elements. Path names with fewer elements will be silently skipped"
    )]
    strip_components: Option<usize>,
    #[arg(
        short = 's',
        value_name = "PATTERN",
        help = "Modify file or archive member names according to the BSD tar -s pattern rules"
    )]
    substitutions: Option<Vec<SubstitutionRule>>,
    #[arg(
        long = "transform",
        visible_alias = "xform",
        value_name = "PATTERN",
        help = "Modify file or archive member names using GNU tar --transform pattern rules"
    )]
    transforms: Option<Vec<TransformRule>>,
    #[arg(
        long,
        help = "Try extracting files with the same ownership as exists in the archive"
    )]
    same_owner: bool,
    #[arg(long, help = "Extract files as yourself")]
    no_same_owner: bool,
    #[arg(
        short = 'o',
        long = "extract-as-self",
        help = "Extract entries as the current user (bsdtar -o equivalent)",
        conflicts_with = "same_owner"
    )]
    extract_as_self: bool,
    #[arg(
        short = 'C',
        long = "cd",
        visible_aliases = ["directory"],
        value_name = "DIRECTORY",
        help = "Change directories after opening the archive but before extracting entries from the archive",
        value_hint = ValueHint::DirPath
    )]
    working_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "chroot() to the current directory after processing any --cd options and before extracting any files"
    )]
    chroot: bool,
    #[arg(
        long,
        help = "Allow extracting symbolic links and hard links that contain root or parent paths"
    )]
    allow_unsafe_links: bool,
    #[command(flatten)]
    pub(crate) file: FileArgsCompat,
}

impl Command for ExtractCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        extract_archive(self)
    }
}
fn extract_archive(args: ExtractCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let start = Instant::now();
    let archive = args.file.archive();
    log::info!("Extract archive {}", archive.display());

    let archives = collect_split_archives(&archive)?;

    let exclude = {
        let mut exclude = args.exclude.unwrap_or_default();
        if let Some(p) = args.exclude_from {
            exclude.extend(read_paths(p, args.null)?);
        }
        if args.exclude_vcs {
            exclude.extend(VCS_FILES.iter().map(|it| String::from(*it)))
        }
        Exclude {
            include: args.include.unwrap_or_default().into(),
            exclude: exclude.into(),
        }
    };

    let keep_permission = args.keep_permission && !args.no_same_permissions;
    let keep_timestamp = if args.modification_time {
        false
    } else {
        args.keep_timestamp
    };
    let keep_xattr = if args.no_keep_xattr { false } else { args.keep_xattr };
    let keep_acl = if args.no_keep_acl { false } else { args.keep_acl };
    let keep_options = KeepOptions {
        keep_timestamp,
        keep_permission,
        keep_xattr,
        keep_acl,
    };
    let owner_options = OwnerOptions::new(
        args.uname,
        args.gname,
        args.uid,
        args.gid,
        args.numeric_owner,
    );
    let same_owner = resolve_same_owner(&args);
    let output_options = OutputOption {
        overwrite: args.overwrite && !args.keep_old_files,
        keep_old_files: args.keep_old_files,
        keep_newer_files: args.keep_newer_files,
        allow_unsafe_links: args.allow_unsafe_links,
        strip_components: args.strip_components,
        out_dir: args.out_dir,
        exclude,
        keep_options,
        owner_options,
        same_owner,
        path_transformers: PathTransformers::new(args.substitutions, args.transforms),
        ignore_zeros: args.ignore_zeros,
        touch_modification_time: args.modification_time,
    };
    if let Some(working_dir) = args.working_dir {
        env::set_current_dir(working_dir)?;
    }
    #[cfg(all(unix, not(target_os = "fuchsia")))]
    if args.chroot {
        std::os::unix::fs::chroot(env::current_dir()?)?;
        env::set_current_dir("/")?;
    }
    #[cfg(not(all(unix, not(target_os = "fuchsia"))))]
    if args.chroot {
        log::warn!("chroot not supported on this platform");
    };
    #[cfg(not(feature = "memmap"))]
    run_extract_archive_reader(
        archives
            .into_iter()
            .map(|it| io::BufReader::with_capacity(64 * 1024, it)),
        args.file.files(),
        || password.as_deref(),
        output_options,
    )?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    #[cfg(feature = "memmap")]
    run_extract_archive(
        archives,
        args.file.files(),
        || password.as_deref(),
        output_options,
    )?;
    log::info!(
        "Successfully extracted an archive in {}",
        DurationDisplay(start.elapsed())
    );
    Ok(())
}

#[derive(Clone, Debug)]
pub(crate) struct OutputOption {
    pub(crate) overwrite: bool,
    pub(crate) keep_old_files: bool,
    pub(crate) keep_newer_files: bool,
    pub(crate) allow_unsafe_links: bool,
    pub(crate) strip_components: Option<usize>,
    pub(crate) out_dir: Option<PathBuf>,
    pub(crate) exclude: Exclude,
    pub(crate) keep_options: KeepOptions,
    pub(crate) owner_options: OwnerOptions,
    pub(crate) same_owner: bool,
    pub(crate) path_transformers: Option<PathTransformers>,
    pub(crate) ignore_zeros: bool,
    pub(crate) touch_modification_time: bool,
}

pub(crate) fn run_extract_archive_reader<'p, Provider>(
    reader: impl IntoIterator<Item = impl Read> + Send,
    files: Vec<String>,
    mut password_provider: Provider,
    args: OutputOption,
) -> anyhow::Result<()>
where
    Provider: FnMut() -> Option<&'p str> + Send,
{
    let password = password_provider();
    let patterns = files;
    let mut globs = GlobPatterns::new(patterns.iter().map(|it| it.as_str()))?;

    let mut link_entries = Vec::new();

    let (tx, rx) = std::sync::mpsc::channel();
    let ignore_zeros = args.ignore_zeros;
    rayon::scope_fifo(|s| -> anyhow::Result<()> {
        run_process_archive_with_options(
            reader,
            password_provider,
            |entry| {
                let item = entry?;
                let item_path = item.header().path().to_string();
                if !globs.is_empty() && !globs.matches_any(&item_path) {
                    log::debug!("Skip: {}", item.header().path());
                    return Ok(());
                }
                if matches!(
                    item.header().data_kind(),
                    DataKind::SymbolicLink | DataKind::HardLink
                ) {
                    link_entries.push(item);
                    return Ok(());
                }
                let tx = tx.clone();
                let args = args.clone();
                s.spawn_fifo(move |_| {
                    tx.send(extract_entry(item, password, &args))
                        .unwrap_or_else(|e| log::error!("{e}: {item_path}"));
                });
                Ok(())
            },
            ignore_zeros,
        )?;
        drop(tx);
        Ok(())
    })?;
    for result in rx {
        result?;
    }
    for item in link_entries {
        extract_entry(item, password, &args)?;
    }

    globs.ensure_all_matched()?;
    Ok(())
}

#[cfg(feature = "memmap")]
pub(crate) fn run_extract_archive<'d, 'p, Provider>(
    archives: impl IntoIterator<Item = &'d [u8]> + Send,
    files: Vec<String>,
    mut password_provider: Provider,
    args: OutputOption,
) -> anyhow::Result<()>
where
    Provider: FnMut() -> Option<&'p str> + Send,
{
    let ignore_zeros = args.ignore_zeros;
    rayon::scope_fifo(|s| {
        let password = password_provider();
        let mut globs = GlobPatterns::new(files.iter().map(|it| it.as_str()))?;

        let mut link_entries = Vec::<NormalEntry>::new();

        let (tx, rx) = std::sync::mpsc::channel();

        run_entries_with_options(
            archives,
            password_provider,
            |entry| {
                let item = entry?;
                let item_path = item.header().path().to_string();
                if !globs.is_empty() && !globs.matches_any(&item_path) {
                    log::debug!("Skip: {}", item.header().path());
                    return Ok(());
                }
                if matches!(
                    item.header().data_kind(),
                    DataKind::SymbolicLink | DataKind::HardLink
                ) {
                    link_entries.push(item.into());
                    return Ok(());
                }
                let tx = tx.clone();
                let args = args.clone();
                s.spawn_fifo(move |_| {
                    tx.send(extract_entry(item, password, &args))
                        .unwrap_or_else(|e| log::error!("{e}: {item_path}"));
                });
                Ok(())
            },
            ignore_zeros,
        )?;
        drop(tx);
        for result in rx {
            result?;
        }

        for item in link_entries {
            extract_entry(item, password, &args)?;
        }
        globs.ensure_all_matched()?;
        Ok(())
    })
}

pub(crate) fn extract_entry<T>(
    item: NormalEntry<T>,
    password: Option<&str>,
    OutputOption {
        overwrite,
        keep_old_files,
        keep_newer_files,
        allow_unsafe_links,
        strip_components,
        out_dir,
        exclude,
        keep_options,
        owner_options,
        same_owner,
        path_transformers,
        ignore_zeros: _,
        touch_modification_time,
    }: &OutputOption,
) -> io::Result<()>
where
    T: AsRef<[u8]>,
    pna::RawChunk<T>: Chunk,
{
    let same_owner = *same_owner;
    let overwrite = *overwrite;
    let keep_old_files = *keep_old_files;
    let keep_newer_files = *keep_newer_files;
    let item_path = item.header().path().as_str();
    if exclude.excluded(item_path) {
        return Ok(());
    }
    let item_path = item.header().path().as_path();
    log::debug!("Extract: {}", item_path.display());
    let item_path = if let Some(strip_count) = *strip_components {
        if item_path.components().count() <= strip_count {
            return Ok(());
        }
        Cow::from(PathBuf::from_iter(item_path.components().skip(strip_count)))
    } else {
        Cow::from(item_path)
    };
    let item_path = if let Some(transformers) = path_transformers {
        Cow::from(PathBuf::from(transformers.apply(
            item_path.to_string_lossy(),
            false,
            false,
        )))
    } else {
        item_path
    };
    let path = if let Some(out_dir) = out_dir {
        Cow::from(out_dir.join(item_path))
    } else {
        item_path
    };
    if path.exists() {
        if keep_old_files {
            log::debug!("Skip {} due to --keep-old-files", path.display());
            return Ok(());
        }
        if keep_newer_files {
            if let Some(entry_mtime) = item.metadata().modified_time() {
                if let Ok(existing_meta) = fs::metadata(&path) {
                    if let Ok(existing_mtime) = existing_meta.modified() {
                        if existing_mtime >= entry_mtime {
                            log::debug!("Skip {} due to --keep-newer-files", path.display());
                            return Ok(());
                        }
                    }
                }
            }
        }
        if !overwrite {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} already exists", path.display()),
            ));
        }
    }
    log::debug!("start: {}", path.display());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let permissions = if keep_options.keep_permission {
        item.metadata()
            .permission()
            .and_then(|p| permissions(p, owner_options))
    } else {
        None
    };
    match item.header().data_kind() {
        DataKind::File => {
            let mut file = utils::fs::file_create(&path, overwrite)?;
            let mut reader = item.reader(ReadOptions::with_password(password))?;
            io::copy(&mut reader, &mut file)?;
            if keep_options.keep_timestamp {
                let mut times = fs::FileTimes::new();
                if let Some(accessed) = item.metadata().accessed_time() {
                    times = times.set_accessed(accessed);
                }
                if let Some(modified) = item.metadata().modified_time() {
                    times = times.set_modified(modified);
                }
                #[cfg(any(windows, target_os = "macos"))]
                if let Some(created) = item.metadata().created_time() {
                    times = times.set_created(created);
                }
                file.set_times(times)?;
            } else if *touch_modification_time {
                let now = SystemTime::now();
                let mut times = fs::FileTimes::new().set_modified(now);
                #[cfg(any(windows, target_os = "macos"))]
                {
                    times = times.set_created(now);
                }
                file.set_times(times)?;
            }
        }
        DataKind::Directory => {
            fs::create_dir_all(&path)?;
        }
        DataKind::SymbolicLink => {
            let reader = item.reader(ReadOptions::with_password(password))?;
            let original = io::read_to_string(reader)?;
            let original = if let Some(substitutions) = path_transformers {
                substitutions.apply(original, true, false)
            } else {
                original
            };
            let original = EntryReference::from_lossy(original);
            if !allow_unsafe_links && is_unsafe_link(&original) {
                log::warn!("Skipped extracting a symbolic link that contains an unsafe link. If you need to extract it, use `--allow-unsafe-links`.");
                return Ok(());
            }
            if overwrite && fs::symlink_metadata(&path).is_ok() {
                utils::fs::remove_path_all(&path)?;
            }
            utils::fs::symlink(original, &path)?;
        }
        DataKind::HardLink => {
            let reader = item.reader(ReadOptions::with_password(password))?;
            let original = io::read_to_string(reader)?;
            let original = if let Some(substitutions) = path_transformers {
                substitutions.apply(original, true, false)
            } else {
                original
            };
            let original = EntryReference::from_lossy(original);
            if !allow_unsafe_links && is_unsafe_link(&original) {
                log::warn!("Skipped extracting a hard link that contains an unsafe link. If you need to extract it, use `--allow-unsafe-links`.");
                return Ok(());
            }
            let mut original = Cow::from(original.as_path());
            if let Some(parent) = path.parent() {
                original = Cow::from(parent.join(original));
            }
            if overwrite && path.exists() {
                utils::fs::remove_path_all(&path)?;
            }
            fs::hard_link(original, &path)?;
        }
    }
    #[cfg(unix)]
    if let Some((p, u, g)) = permissions {
        use std::os::unix::fs::PermissionsExt;
        if same_owner {
            match chown(&path, u, g) {
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                    log::warn!("failed to restore owner of {}: {}", path.display(), e)
                }
                r => r?,
            }
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(p.permissions().into()))?;
    };
    #[cfg(windows)]
    if let Some((p, u, g)) = permissions {
        if same_owner {
            chown(&path, u, g)?;
        }
        utils::os::windows::fs::chmod(&path, p.permissions())?;
    }
    #[cfg(not(any(unix, windows)))]
    if let Some(_) = permissions {
        log::warn!("Currently permission is not supported on this platform.");
    }
    #[cfg(unix)]
    if keep_options.keep_xattr {
        utils::os::unix::fs::xattrs::set_xattrs(&path, item.xattrs())?;
    }
    #[cfg(not(unix))]
    if keep_options.keep_xattr {
        log::warn!("Currently extended attribute is not supported on this platform.");
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
            use crate::chunk::{acl_convert_current_platform, AcePlatform, Acl};
            use crate::ext::*;
            use itertools::Itertools;

            let platform = AcePlatform::CURRENT;
            let acls = item.acl()?;
            if let Some((platform, acl)) = acls.into_iter().find_or_first(|(p, _)| p.eq(&platform))
            {
                if !acl.is_empty() {
                    utils::acl::set_facl(
                        &path,
                        acl_convert_current_platform(Acl {
                            platform,
                            entries: acl,
                        }),
                    )?;
                }
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
    log::debug!("end: {}", path.display());
    Ok(())
}

fn permissions<'p>(
    permission: &'p Permission,
    owner_options: &'_ OwnerOptions,
) -> Option<(&'p Permission, Option<User>, Option<Group>)> {
    let user = if let Some(uid) = owner_options.uid {
        User::from_uid(uid.into())
    } else {
        search_owner(
            owner_options.uname.as_deref().unwrap_or(permission.uname()),
            permission.uid(),
        )
    };
    let group = if let Some(gid) = owner_options.gid {
        Group::from_gid(gid.into())
    } else {
        search_group(
            owner_options.gname.as_deref().unwrap_or(permission.gname()),
            permission.gid(),
        )
    };
    Some((permission, user.ok(), group.ok()))
}

fn search_owner(name: &str, id: u64) -> io::Result<User> {
    let user = User::from_name(name);
    if user.is_ok() {
        return user;
    }
    User::from_uid((id as u32).into())
}

fn search_group(name: &str, id: u64) -> io::Result<Group> {
    let group = Group::from_name(name);
    if group.is_ok() {
        return group;
    }
    Group::from_gid((id as u32).into())
}

fn resolve_same_owner(args: &ExtractCommand) -> bool {
    if args.same_owner {
        true
    } else if args.no_same_owner || args.extract_as_self {
        false
    } else {
        default_same_owner()
    }
}

#[cfg(unix)]
fn default_same_owner() -> bool {
    geteuid().as_raw() == 0
}

#[cfg(windows)]
const fn default_same_owner() -> bool {
    false
}

#[cfg(not(any(unix, windows)))]
const fn default_same_owner() -> bool {
    false
}

#[inline]
fn is_unsafe_link(reference: &EntryReference) -> bool {
    reference.as_path().components().any(|it| {
        matches!(
            it,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}
