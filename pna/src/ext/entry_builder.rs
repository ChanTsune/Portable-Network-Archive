use crate::ext::private;
use libpna::{EntryBuilder, Metadata};

/// [`EntryBuilder`] extension trait.
pub trait EntryBuilderExt: private::Sealed {
    /// Set metadata to entry.
    fn add_metadata(&mut self, metadata: &Metadata);
}

impl EntryBuilderExt for EntryBuilder {
    /// Set metadata to entry.
    #[inline]
    fn add_metadata(&mut self, metadata: &Metadata) {
        if let Some(created) = metadata.created() {
            self.created(created);
        }
        if let Some(modified) = metadata.modified() {
            self.modified(modified);
        }
        if let Some(accessed) = metadata.accessed() {
            self.accessed(accessed);
        }
        if let Some(permission) = metadata.permission() {
            self.permission(permission.clone());
        }
    }
}
