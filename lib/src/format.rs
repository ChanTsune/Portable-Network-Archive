//! Core definitions for the PNA file format.

mod signature;

pub use signature::PNA_SIGNATURE;
pub(crate) use signature::validate_signature;
