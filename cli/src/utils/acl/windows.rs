use crate::chunk;
use std::io;
use std::path::Path;

pub fn set_acl(path: &Path, acl: Vec<chunk::Ace>) -> io::Result<()> {
    let acl: Vec<windows_acl::acl::ACLEntry> = acl.into_iter().map(Into::into).collect();
    let mut current_acl = windows_acl::acl::ACL::from_file_path(&path.to_string_lossy(), false)
        .map_err(io::Error::other)?;
    for e in current_acl.all().map_err(io::Error::other)? {
        e.mask;
    }
    todo!()
}

pub fn get_acl(path: &Path) -> io::Result<Vec<chunk::Ace>> {
    let acl = windows_acl::acl::ACL::from_file_path(&path.to_string_lossy(), false)
        .map_err(io::Error::other)?;
    let ace_list = acl.all().map_err(io::Error::other)?;
    Ok(ace_list.into_iter().map(Into::into).collect())
}
