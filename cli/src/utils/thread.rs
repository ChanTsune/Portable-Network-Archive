use rayon::{ThreadPool, ThreadPoolBuilder};
use std::io;

pub(crate) fn build() -> io::Result<ThreadPool> {
    if cfg!(target_os = "wasi") {
        ThreadPoolBuilder::default().use_current_thread()
    } else {
        ThreadPoolBuilder::default()
    }
    .build()
    .map_err(io::Error::other)
}
