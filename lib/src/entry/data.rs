/// Data part of entry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryData {
    pub(crate) data: Vec<Vec<u8>>,
    pub(crate) phsf: Option<String>,
}

impl EntryData {
    /// Returns sum of `FDAT` chunks data length.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.data.iter().map(|it| it.len()).sum()
    }
}
