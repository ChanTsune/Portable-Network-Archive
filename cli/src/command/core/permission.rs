/// Override values for ownership during permission operations.
/// During creation: Override values stored in archive (None = use filesystem)
/// During extraction: Override values restored from archive (None = use archive)
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct OwnerOptions {
    pub(crate) uname: Option<String>,
    pub(crate) gname: Option<String>,
    pub(crate) uid: Option<u32>,
    pub(crate) gid: Option<u32>,
}

/// How to handle file mode bits (permissions).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum ModeStrategy {
    /// Don't preserve mode bits
    #[default]
    Never,
    /// Preserve mode bits from source
    Preserve,
}

/// How to handle file ownership (uid/gid/uname/gname).
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum OwnerStrategy {
    /// Don't restore ownership
    #[default]
    Never,
    /// Restore ownership with optional overrides
    Preserve { options: OwnerOptions },
}
