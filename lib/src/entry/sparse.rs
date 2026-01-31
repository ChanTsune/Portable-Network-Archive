//! Sparse file map support.
//!
//! This module provides types for representing sparse file metadata in PNA archives.
//! A sparse file contains "holes" - regions that read as zeros but don't occupy disk space.

use std::io;

/// A region of actual data in a sparse file.
///
/// Represents a contiguous block of data at a specific offset within the logical file.
/// The data for this region is stored contiguously in the archive's FDAT chunks.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DataRegion {
    offset: u64,
    size: u64,
}

impl DataRegion {
    /// Creates a new data region.
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset in the logical file where this data region starts
    /// * `size` - Size of the data region in bytes
    #[inline]
    pub const fn new(offset: u64, size: u64) -> Self {
        Self { offset, size }
    }

    /// Returns the byte offset in the logical file.
    #[inline]
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the size of the data region in bytes.
    #[inline]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Returns the exclusive end offset: `offset + size`.
    #[inline]
    pub const fn end(&self) -> u64 {
        self.offset + self.size
    }
}

/// Sparse file map describing data regions within a file.
///
/// The map contains the logical file size and an ordered list of data regions.
/// Gaps between data regions are holes that read as zeros.
///
/// # SPAR Chunk Format
///
/// ```text
/// +------------------+------------------+------------------+-----+
/// | logical_size     | offset_0         | size_0           | ... |
/// | (8 bytes, u64)   | (8 bytes, u64)   | (8 bytes, u64)   |     |
/// +------------------+------------------+------------------+-----+
/// ```
///
/// All values are unsigned 64-bit integers in big-endian byte order.
///
/// # Invariants
///
/// - Regions are sorted by offset in ascending order
/// - Regions do not overlap: `regions[i].end() <= regions[i+1].offset()`
/// - All regions are within bounds: `region.end() <= logical_size`
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SparseMap {
    logical_size: u64,
    regions: Vec<DataRegion>,
}

impl SparseMap {
    /// Creates a new sparse map.
    ///
    /// # Arguments
    ///
    /// * `logical_size` - The total logical size of the file (including holes)
    /// * `regions` - Data regions in ascending offset order
    ///
    /// # Panics
    ///
    /// In debug builds, panics if regions violate invariants:
    /// - Not sorted by offset
    /// - Overlapping regions
    /// - Region extends beyond logical_size
    #[inline]
    pub fn new(logical_size: u64, regions: Vec<DataRegion>) -> Self {
        #[cfg(debug_assertions)]
        Self::validate_regions(logical_size, &regions);
        Self {
            logical_size,
            regions,
        }
    }

    #[cfg(debug_assertions)]
    fn validate_regions(logical_size: u64, regions: &[DataRegion]) {
        for i in 1..regions.len() {
            debug_assert!(
                regions[i - 1].offset < regions[i].offset,
                "regions must be sorted by offset in ascending order: {} >= {}",
                regions[i - 1].offset,
                regions[i].offset
            );
            debug_assert!(
                regions[i - 1].end() <= regions[i].offset,
                "regions must not overlap: region {} ends at {}, region {} starts at {}",
                i - 1,
                regions[i - 1].end(),
                i,
                regions[i].offset
            );
        }
        if let Some(last) = regions.last() {
            debug_assert!(
                last.end() <= logical_size,
                "region must be within logical size: region ends at {}, logical size is {}",
                last.end(),
                logical_size
            );
        }
    }

    /// Returns the logical file size (total size including holes).
    #[inline]
    pub const fn logical_size(&self) -> u64 {
        self.logical_size
    }

    /// Returns the data regions.
    #[inline]
    pub fn regions(&self) -> &[DataRegion] {
        &self.regions
    }

    /// Returns the total size of all data regions (actual data, excludes holes).
    ///
    /// This is the amount of data stored in the archive's FDAT chunks.
    #[inline]
    pub fn data_size(&self) -> u64 {
        self.regions.iter().map(|r| r.size).sum()
    }

    /// Returns `true` if there are no data regions (entire file is a hole).
    #[inline]
    pub fn is_all_hole(&self) -> bool {
        self.regions.is_empty()
    }

    /// Parses a sparse map from SPAR chunk data.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Data is too small (less than 8 bytes)
    /// - Data size is not 8 + n*16 bytes
    /// - Regions are not in ascending offset order
    /// - Regions overlap
    /// - A region extends beyond the logical size
    ///
    /// # Panics
    ///
    /// This function does not panic; all slice operations are bounds-checked
    /// by the length validations above.
    #[inline]
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "SPAR chunk too small: {} bytes, expected at least 8",
                    data.len()
                ),
            ));
        }
        if !(data.len() - 8).is_multiple_of(16) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "SPAR chunk has invalid size: {} bytes (expected 8 + n*16)",
                    data.len()
                ),
            ));
        }

        // SAFETY: length checks above guarantee these slices are exactly 8 bytes
        let logical_size = u64::from_be_bytes(data[0..8].try_into().expect("checked"));
        let entry_count = (data.len() - 8) / 16;
        let mut regions = Vec::with_capacity(entry_count);

        for i in 0..entry_count {
            let base = 8 + i * 16;
            // SAFETY: is_multiple_of check guarantees these slices exist
            let offset = u64::from_be_bytes(data[base..base + 8].try_into().expect("checked"));
            let size = u64::from_be_bytes(data[base + 8..base + 16].try_into().expect("checked"));
            regions.push(DataRegion::new(offset, size));
        }

        // Validate constraints per SPAR specification
        for i in 1..regions.len() {
            if regions[i - 1].offset >= regions[i].offset {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "SPAR regions must be in ascending offset order: \
                         region {} offset {} >= region {} offset {}",
                        i - 1,
                        regions[i - 1].offset,
                        i,
                        regions[i].offset
                    ),
                ));
            }
            if regions[i - 1].end() > regions[i].offset {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "SPAR regions must not overlap: \
                         region {} ends at {}, region {} starts at {}",
                        i - 1,
                        regions[i - 1].end(),
                        i,
                        regions[i].offset
                    ),
                ));
            }
        }
        if let Some(last) = regions.last()
            && last.end() > logical_size
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "SPAR region exceeds logical size: \
                     region ends at {}, logical size is {}",
                    last.end(),
                    logical_size
                ),
            ));
        }

        Ok(Self {
            logical_size,
            regions,
        })
    }

    /// Serializes the sparse map to SPAR chunk data.
    ///
    /// The returned bytes can be used as the data portion of a SPAR chunk.
    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(8 + self.regions.len() * 16);
        data.extend_from_slice(&self.logical_size.to_be_bytes());
        for region in &self.regions {
            data.extend_from_slice(&region.offset.to_be_bytes());
            data.extend_from_slice(&region.size.to_be_bytes());
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn data_region_accessors() {
        let region = DataRegion::new(100, 50);
        assert_eq!(region.offset(), 100);
        assert_eq!(region.size(), 50);
        assert_eq!(region.end(), 150);
    }

    #[test]
    fn data_region_zero_size() {
        let region = DataRegion::new(100, 0);
        assert_eq!(region.offset(), 100);
        assert_eq!(region.size(), 0);
        assert_eq!(region.end(), 100);
    }

    #[test]
    fn sparse_map_round_trip() {
        let map = SparseMap::new(
            1000,
            vec![DataRegion::new(0, 100), DataRegion::new(500, 200)],
        );
        let bytes = map.to_bytes();
        let parsed = SparseMap::from_bytes(&bytes).unwrap();
        assert_eq!(map, parsed);
    }

    #[test]
    fn sparse_map_empty_regions() {
        let map = SparseMap::new(1000, vec![]);
        assert!(map.is_all_hole());
        assert_eq!(map.data_size(), 0);
        assert_eq!(map.logical_size(), 1000);

        let bytes = map.to_bytes();
        assert_eq!(bytes.len(), 8); // Only logical_size

        let parsed = SparseMap::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.logical_size(), 1000);
        assert!(parsed.regions().is_empty());
    }

    #[test]
    fn sparse_map_single_region_at_start() {
        let map = SparseMap::new(1000, vec![DataRegion::new(0, 100)]);
        assert!(!map.is_all_hole());
        assert_eq!(map.data_size(), 100);

        let bytes = map.to_bytes();
        assert_eq!(bytes.len(), 8 + 16); // logical_size + 1 region

        let parsed = SparseMap::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.regions().len(), 1);
        assert_eq!(parsed.regions()[0].offset(), 0);
        assert_eq!(parsed.regions()[0].size(), 100);
    }

    #[test]
    fn sparse_map_data_size() {
        let map = SparseMap::new(
            1000,
            vec![DataRegion::new(0, 100), DataRegion::new(500, 200)],
        );
        assert_eq!(map.data_size(), 300);
    }

    #[test]
    fn sparse_map_adjacent_regions() {
        // Adjacent but not overlapping: [0-100] and [100-200]
        let map = SparseMap::new(
            200,
            vec![DataRegion::new(0, 100), DataRegion::new(100, 100)],
        );
        assert_eq!(map.data_size(), 200);

        let bytes = map.to_bytes();
        let parsed = SparseMap::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.regions().len(), 2);
    }

    #[test]
    fn sparse_map_invalid_order() {
        // Regions out of order: 500 before 0
        #[rustfmt::skip]
        let data = [
            0, 0, 0, 0, 0, 0, 3, 232, // logical_size = 1000
            0, 0, 0, 0, 0, 0, 1, 244, // offset = 500
            0, 0, 0, 0, 0, 0, 0, 100, // size = 100
            0, 0, 0, 0, 0, 0, 0, 0,   // offset = 0 (out of order!)
            0, 0, 0, 0, 0, 0, 0, 100, // size = 100
        ];
        let result = SparseMap::from_bytes(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ascending"));
    }

    #[test]
    fn sparse_map_overlapping_regions() {
        // Regions overlap: [0-100] and [50-150]
        #[rustfmt::skip]
        let data = [
            0, 0, 0, 0, 0, 0, 3, 232, // logical_size = 1000
            0, 0, 0, 0, 0, 0, 0, 0,   // offset = 0
            0, 0, 0, 0, 0, 0, 0, 100, // size = 100 (ends at 100)
            0, 0, 0, 0, 0, 0, 0, 50,  // offset = 50 (overlaps!)
            0, 0, 0, 0, 0, 0, 0, 100, // size = 100
        ];
        let result = SparseMap::from_bytes(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("overlap"));
    }

    #[test]
    fn sparse_map_exceeds_logical_size() {
        // Region exceeds logical size
        #[rustfmt::skip]
        let data = [
            0, 0, 0, 0, 0, 0, 0, 100, // logical_size = 100
            0, 0, 0, 0, 0, 0, 0, 50,  // offset = 50
            0, 0, 0, 0, 0, 0, 0, 100, // size = 100 (ends at 150 > 100)
        ];
        let result = SparseMap::from_bytes(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds"));
    }

    #[test]
    fn sparse_map_too_small() {
        let data = [0, 0, 0, 0]; // Less than 8 bytes
        let result = SparseMap::from_bytes(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too small"));
    }

    #[test]
    fn sparse_map_invalid_length() {
        // 11 bytes: not 8 + n*16
        let data = [0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0];
        let result = SparseMap::from_bytes(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid size"));
    }

    #[test]
    fn sparse_map_zero_logical_size_empty_regions() {
        let map = SparseMap::new(0, vec![]);
        assert!(map.is_all_hole());
        assert_eq!(map.logical_size(), 0);

        let bytes = map.to_bytes();
        let parsed = SparseMap::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.logical_size(), 0);
    }
}
