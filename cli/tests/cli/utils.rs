pub mod archive;
pub mod diff;
pub mod time;

use std::{borrow::Cow, fs, io, path::Path};

#[derive(rust_embed::Embed)]
#[folder = "../resources/test"]
pub struct TestResources;

#[derive(rust_embed::Embed)]
#[folder = "../lib"]
pub struct LibSourceCode;

pub trait EmbedExt {
    fn extract_all(into: impl AsRef<Path>) -> io::Result<()>;
    fn extract_in(item: &str, into: impl AsRef<Path>) -> io::Result<()>;
    fn item_iter() -> impl Iterator<Item = (Cow<'static, str>, rust_embed::EmbeddedFile)>;
}

impl<T: rust_embed::Embed> EmbedExt for T {
    fn extract_all(into: impl AsRef<Path>) -> io::Result<()> {
        extract_all::<Self>(into)
    }
    fn extract_in(item: &str, into: impl AsRef<Path>) -> io::Result<()> {
        extract_in::<Self>(item, into)
    }
    fn item_iter() -> impl Iterator<Item = (Cow<'static, str>, rust_embed::EmbeddedFile)> {
        item_iter::<Self>()
    }
}

pub fn item_iter<T: rust_embed::Embed>()
-> impl Iterator<Item = (Cow<'static, str>, rust_embed::EmbeddedFile)> {
    T::iter().flat_map(|i| T::get(&i).map(|embedded| (i, embedded)))
}

pub fn extract_all<T: rust_embed::Embed>(into: impl AsRef<Path>) -> io::Result<()> {
    let path = into.as_ref();
    T::iter().try_for_each(|i| {
        let path = path.join(i.as_ref());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, T::get(&i).unwrap().data)
    })
}

pub fn extract_in<T: rust_embed::Embed>(item: &str, into: impl AsRef<Path>) -> io::Result<()> {
    let path = into.as_ref();
    if let Some(b) = T::get(item) {
        let path = path.join(item);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, b.data)?;
        return Ok(());
    }
    T::iter().try_for_each(|i| {
        if i.starts_with(item) {
            let path = path.join(i.as_ref());
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, T::get(&i).unwrap().data)?;
        }
        Ok(())
    })
}

pub fn setup() {
    fs::create_dir_all(env!("CARGO_TARGET_TMPDIR")).expect("Failed to create working dir");
    std::env::set_current_dir(env!("CARGO_TARGET_TMPDIR")).expect("Failed to set current dir");
}

/// Environment variable listing capabilities (comma-separated) whose absence
/// is a failure rather than a skip. CI sets it per job so that a precondition
/// which should hold on that runner cannot silently turn a test green.
pub const REQUIRE_ENV: &str = "PNA_TEST_REQUIRE";

pub fn is_required(list: &str, capability: &str) -> bool {
    list.split(',').map(str::trim).any(|c| c == capability)
}

#[track_caller]
pub fn skip_or_fail(capability: &str, condition: &str) {
    let list = std::env::var(REQUIRE_ENV).unwrap_or_default();
    if is_required(&list, capability) {
        panic!("{REQUIRE_ENV} lists `{capability}` but `{condition}` is false");
    }
    eprintln!("skipped: `{capability}` unavailable (`{condition}` is false)");
}

/// Leaves the current test when a runtime-only precondition is unmet.
/// Capability names are the vocabulary of `PNA_TEST_REQUIRE`; keep them
/// stable: `birthtime`, `mtime_nanos`, `xattr`, `nodump`, `chmod`, `setuid`,
/// `root`, `unprivileged`, `mount`.
macro_rules! skip_unless {
    ($capability:literal, $cond:expr) => {
        if !$cond {
            $crate::utils::skip_or_fail($capability, stringify!($cond));
            return;
        }
    };
}

/// `false` only when the OS refuses the change; any other error is a test bug.
#[cfg(unix)]
#[track_caller]
pub fn set_mode(path: impl AsRef<Path>, mode: u32) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match fs::set_permissions(path, fs::Permissions::from_mode(mode)) {
        Ok(()) => true,
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => false,
        Err(e) => panic!("set_permissions: {e}"),
    }
}

/// Non-mutating probe: reading an absent attribute is `Ok(None)` on a
/// filesystem with xattr support and `Unsupported` otherwise.
///
/// Ask this rather than `xattr::SUPPORTED_PLATFORM`, which is a compile-time
/// constant and stays true on FreeBSD and NetBSD whether or not the mounted
/// file system carries extended attributes.
#[cfg(unix)]
#[track_caller]
pub fn fs_supports_xattr(path: impl AsRef<Path>) -> bool {
    match xattr::get(path, "user.pna_test_probe") {
        Ok(_) => true,
        Err(e) if e.kind() == io::ErrorKind::Unsupported => false,
        Err(e) => panic!("xattr::get: {e}"),
    }
}

/// Spawns the `pna` binary under `mask` without touching this test process's
/// own umask, which `libc::umask` would otherwise change process-wide (it is
/// not thread-local) and race with unrelated tests running concurrently in
/// this binary.
#[cfg(unix)]
pub fn pna_cmd_with_umask(mask: u16) -> assert_cmd::Command {
    use assert_cmd::cargo::CommandCargoExt as _;
    use std::os::unix::process::CommandExt as _;

    let mut cmd = std::process::Command::cargo_bin("pna").expect("pna binary not found");
    // SAFETY: umask() only runs in the forked child, after fork() and before
    // exec(), so it affects only that child process and never the parent
    // test runner or its other threads.
    unsafe {
        cmd.pre_exec(move || {
            libc::umask(mask as libc::mode_t);
            Ok(())
        });
    }
    assert_cmd::Command::from_std(cmd)
}

/// Record separator that the bsdtar-compat list output is expected to emit on
/// the host platform. Reference bsdtar relies on the C runtime's text-mode
/// translation, so Windows list output ends each record with CRLF while every
/// other platform keeps a bare LF.
pub fn list_record_separator() -> &'static str {
    if cfg!(target_os = "windows") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Build the expected stdout payload for a list command by joining each record
/// with the platform's record separator and terminating the final record with
/// the same separator.
pub fn list_lines(records: &[&str]) -> String {
    let separator = list_record_separator();
    let mut out = String::with_capacity(
        records.iter().map(|r| r.len()).sum::<usize>() + separator.len() * records.len(),
    );
    for record in records {
        out.push_str(record);
        out.push_str(separator);
    }
    out
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn remove_with_empty_parents(path: impl AsRef<Path>) -> io::Result<()> {
    fn inner(path: &Path) -> io::Result<()> {
        pna::fs::remove_path_all(path)?;
        let mut current_path = path;
        while let Some(dir) = current_path.parent() {
            if fs::read_dir(dir)?.next().is_none() {
                fs::remove_dir(dir)?;
                current_path = dir;
            } else {
                break;
            }
        }
        Ok(())
    }
    inner(path.as_ref())
}

#[cfg(test)]
mod tests {
    use super::is_required;

    #[test]
    fn is_required_matches_whole_comma_separated_items() {
        let cases = [
            ("", "xattr", false),
            ("xattr", "xattr", true),
            ("birthtime, xattr ,root", "xattr", true),
            ("xattrs", "xattr", false),
            ("xattr", "xattrs", false),
        ];
        for (list, capability, expected) in cases {
            assert_eq!(
                is_required(list, capability),
                expected,
                "{list:?} / {capability}"
            );
        }
    }
}
