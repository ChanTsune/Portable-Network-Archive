/// Helper macro to skip test if we don't have root privileges.
#[macro_export]
macro_rules! skip_if_not_root {
    () => {
        if !nix::unistd::Uid::effective().is_root() {
            eprintln!("Skipping test: requires root privileges");
            return;
        }
    };
}
pub use skip_if_not_root;
