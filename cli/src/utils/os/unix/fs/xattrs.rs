use pna::ExtendedAttribute;
use std::io;
use std::path::Path;

pub(crate) fn get_xattrs<P: AsRef<Path>>(path: P) -> io::Result<Vec<ExtendedAttribute>> {
    fn inner(path: &Path) -> io::Result<Vec<ExtendedAttribute>> {
        let mut xattrs = Vec::new();
        for name in xattr::list(path)? {
            let value = xattr::get(path, &name)?.unwrap_or_default();
            xattrs.push(ExtendedAttribute::new(name.to_string_lossy().into(), value))
        }
        Ok(xattrs)
    }
    if xattr::SUPPORTED_PLATFORM {
        inner(path.as_ref())
    } else {
        log::warn!("Currently extended attribute is not supported on this platform.");
        Ok(Vec::new())
    }
}

pub(crate) fn set_xattrs<P: AsRef<Path>>(path: P, xattrs: &[ExtendedAttribute]) -> io::Result<()> {
    fn inner(path: &Path, xattrs: &[ExtendedAttribute]) -> io::Result<()> {
        for x in xattrs {
            xattr::set(path, x.name(), x.value())?;
        }
        Ok(())
    }
    if xattr::SUPPORTED_PLATFORM {
        inner(path.as_ref(), xattrs)
    } else {
        log::warn!("Currently extended attribute is not supported on this platform.");
        Ok(())
    }
}
