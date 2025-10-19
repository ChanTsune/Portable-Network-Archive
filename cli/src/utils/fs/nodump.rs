#[cfg(any(target_os = "linux", target_os = "android"))]
mod platform {
    use nix::{
        fcntl::{OFlag, open},
        ioctl_read_bad,
        sys::stat::Mode,
    };
    use std::os::fd::AsRawFd;
    use std::{io, path::Path};

    const FS_NODUMP_FL: libc::c_int = 0x00000040;

    ioctl_read_bad!(fs_ioc_getflags, libc::FS_IOC_GETFLAGS, libc::c_int);

    pub(crate) fn is_nodump(path: &Path) -> io::Result<bool> {
        let fd = open(path, OFlag::O_RDONLY | OFlag::O_NOFOLLOW, Mode::empty())?;
        let mut flags: libc::c_int = 0;
        unsafe { fs_ioc_getflags(fd.as_raw_fd(), &mut flags) }?;
        Ok((flags & FS_NODUMP_FL) != 0)
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
mod platform {
    use nix::sys::stat::lstat;
    use std::{io, path::Path};

    pub(crate) fn is_nodump(path: &Path) -> io::Result<bool> {
        let stat = lstat(path)?;
        Ok((stat.st_flags & libc::UF_NODUMP as libc::c_uint) != 0)
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd"
)))]
mod platform {
    use std::{io, path::Path};

    pub(crate) fn is_nodump(_path: &Path) -> io::Result<bool> {
        Ok(false)
    }
}

pub(crate) use platform::is_nodump;
