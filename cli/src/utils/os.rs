#[cfg(target_os = "redox")]
mod redox {
    pub(crate) mod fs {
        pub(crate) mod owner;
    }
}
#[cfg(unix)]
pub(crate) mod unix;
#[cfg(windows)]
pub(crate) mod windows;
