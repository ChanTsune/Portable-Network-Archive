/// How to handle ownership during permission operations.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum OwnerSource {
    /// Don't restore ownership (extraction only - same as !same_owner)
    /// During extraction: chmod() is called but lchown() is NOT
    NoRestore,
    /// Use ownership from source with optional overrides.
    /// During creation: Override values stored in archive (None = use filesystem)
    /// During extraction: Override values restored from archive (None = use archive)
    FromSource {
        uname: Option<String>,
        gname: Option<String>,
        uid: Option<u32>,
        gid: Option<u32>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum PermissionStrategy {
    Never,
    /// Preserve permissions with configurable ownership handling
    Preserve {
        owner: OwnerSource,
    },
}
