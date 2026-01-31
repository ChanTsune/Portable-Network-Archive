#[cfg(feature = "acl")]
pub(crate) mod acl;
pub(crate) mod env;
pub(crate) mod fmt;
pub(crate) mod fs;
mod globs;
pub(crate) mod io;
#[cfg(feature = "memmap")]
pub(crate) mod mmap;
pub(crate) mod os;
mod path;
pub(crate) mod process;
pub(crate) mod sparse;
pub(crate) mod str;

pub(crate) use {globs::*, path::*};

/// Version Control System file names.
pub(crate) const VCS_FILES: &[&str] = &[
    // CVS
    "CVS",
    ".cvsignore",
    // RCS
    "RCS",
    // SCCS
    "SCCS",
    // SVN
    ".svn",
    // git
    ".git",
    ".gitignore",
    ".gitattributes",
    ".gitmodules",
    // Arch
    ".arch-ids",
    "{arch}",
    "=RELEASE-ID",
    "=meta-update",
    "=update",
    // Bazaar
    ".bzr",
    ".bzrignore",
    ".bzrtags",
    // Mercurial
    ".hg",
    ".hgignore",
    ".hgtags",
    // darcs
    "_darcs",
];
