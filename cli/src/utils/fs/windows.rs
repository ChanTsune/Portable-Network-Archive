use crate::utils::os::windows::security::{SecurityDescriptor, Sid};
use std::io;
use std::path::Path;

pub(crate) fn change_owner(path: &Path, owner: Option<Sid>, group: Option<Sid>) -> io::Result<()> {
    let sd = SecurityDescriptor::try_from(path)?;
    sd.apply(
        owner.and_then(|it| it.to_psid().ok()),
        group.and_then(|it| it.to_psid().ok()),
        None,
    )
}
