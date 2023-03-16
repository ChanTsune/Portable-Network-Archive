/// MetaData information about a entry
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Metadata {
    pub(crate) compressed_size: usize,
}

impl Metadata {
    /// Compressed size of entry data
    #[inline]
    pub fn compressed_size(&self) -> usize {
        self.compressed_size
    }
}
