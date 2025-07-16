#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum AclPlatform {
    Posix,
    Mac,
    Windows,
    Unknown(u8)
}
