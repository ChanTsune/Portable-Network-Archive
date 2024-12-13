use rayon::{ThreadPool, ThreadPoolBuilder};
use std::io;

pub(crate) fn build() -> io::Result<ThreadPool> {
    ThreadPoolBuilder::default()
        .build()
        .map_err(io::Error::other)
}
