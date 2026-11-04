/// Returns true if the current process is running as root (effective UID == 0).
pub(crate) fn is_running_as_root() -> bool {
    nix::unistd::Uid::effective().is_root()
}
