use std::time::Duration;

/// MetaData information about a entry
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Metadata {
    pub(crate) compressed_size: usize,
    pub(crate) created: Option<Duration>,
    pub(crate) modified: Option<Duration>,
}

impl Metadata {
    /// Compressed size of entry data
    #[inline]
    pub fn compressed_size(&self) -> usize {
        self.compressed_size
    }
    /// Created time since unix epoch time of entry
    #[inline]
    pub fn created(&self) -> Option<Duration> {
        self.created
    }
    /// Modified time since unix epoch time of entry
    #[inline]
    pub fn modified(&self) -> Option<Duration> {
        self.modified
    }
}
