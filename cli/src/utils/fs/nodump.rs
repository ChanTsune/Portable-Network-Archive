use crate::utils::fs::get_flags;
use std::{io, path::Path};

/// Check if the file has the nodump flag set.
/// This is used to skip files during backup operations.
pub(crate) fn is_nodump(path: &Path) -> io::Result<bool> {
    let flags = get_flags(path)?;
    Ok(flags.iter().any(|f| f == "nodump"))
}
