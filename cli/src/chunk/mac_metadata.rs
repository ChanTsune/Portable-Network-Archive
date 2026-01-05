use pna::ChunkType;

/// Private chunk type for macOS metadata (AppleDouble format).
/// Name follows PNA chunk naming convention where case has semantic meaning:
/// - lowercase first letter: ancillary (not critical)
/// - lowercase second letter: private (not public)
/// - uppercase third letter: reserved
/// - lowercase fourth letter: safe to copy
#[allow(non_upper_case_globals)]
pub const maMd: ChunkType = unsafe { ChunkType::from_unchecked(*b"maMd") };
