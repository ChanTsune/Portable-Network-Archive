# Sparse File Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement SPAR chunk support for sparse file archiving and restoration in PNA.

**Architecture:** Add SPAR as a new Critical chunk type in libpna. Modify entry parsing to reject unknown Critical chunks (PNG compliance) and parse SPAR. CLI detects sparse files on creation and restores sparse structure on extraction.

**Tech Stack:** Rust, libpna, libc (for SEEK_HOLE/SEEK_DATA, fallocate)

**Specification:** See `docs/plans/2026-01-31-sparse-file-support-design.md`

---

## Term 0: Prerequisites

### Task 0-1: Reject Unknown Critical Chunks in NormalEntry and SolidEntry

**Purpose:** PNG仕様に従い、未知のCriticalチャンクを拒否する。これがないと古いリーダーがSPAR付きエントリを破損データとして展開してしまう。

**Files:**
- Modify: `lib/src/entry.rs`
  - Line 660: NormalEntry の `_ => extra.push(chunk)` を修正
  - Line 562: SolidEntry の `_ => extra.push(chunk)` を修正

#### Step 1: Write failing tests

Add to `lib/src/entry.rs` test module (after line 1232):

```rust
#[test]
fn reject_unknown_critical_chunk_in_normal_entry() {
    // Unknown Critical chunk: uppercase first letter = Critical
    let unknown_critical = RawChunk::from_data(
        unsafe { ChunkType::from_unchecked(*b"XUNK") },
        vec![1, 2, 3],
    );
    // Minimal valid FHED: version 0.0, kind=File(0), compression=No(0), encryption=No(0), cipher_mode=0
    let fhed = RawChunk::from_data(ChunkType::FHED, vec![0, 0, 0, 0, 0, 0]);
    let fend = RawChunk::from_data(ChunkType::FEND, vec![]);

    let raw_entry = RawEntry(vec![fhed, unknown_critical, fend]);
    let result = NormalEntry::try_from(raw_entry);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("critical"));
}

#[test]
fn reject_unknown_critical_chunk_in_solid_entry() {
    let unknown_critical = RawChunk::from_data(
        unsafe { ChunkType::from_unchecked(*b"XUNK") },
        vec![1, 2, 3],
    );
    // Minimal valid SHED: version 0.0, compression=No(0), encryption=No(0), cipher_mode=0
    let shed = RawChunk::from_data(ChunkType::SHED, vec![0, 0, 0, 0, 0]);
    let send = RawChunk::from_data(ChunkType::SEND, vec![]);

    let raw_entry = RawEntry(vec![shed, unknown_critical, send]);
    let result = SolidEntry::try_from(raw_entry);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("critical"));
}

#[test]
fn accept_unknown_ancillary_chunk_in_normal_entry() {
    // Unknown Ancillary chunk: lowercase first letter = Ancillary
    let unknown_ancillary = RawChunk::from_data(
        unsafe { ChunkType::from_unchecked(*b"xUNK") },
        vec![1, 2, 3],
    );
    let fhed = RawChunk::from_data(ChunkType::FHED, vec![0, 0, 0, 0, 0, 0]);
    let fend = RawChunk::from_data(ChunkType::FEND, vec![]);

    let raw_entry = RawEntry(vec![fhed, unknown_ancillary, fend]);
    let result = NormalEntry::try_from(raw_entry);

    // Ancillary chunks should be accepted and stored in extra
    assert!(result.is_ok());
    let entry = result.unwrap();
    assert_eq!(entry.extra.len(), 1);
}
```

#### Step 2: Run tests to verify they fail

```bash
cargo test -p libpna --lib entry::tests::reject_unknown_critical_chunk_in_normal_entry
cargo test -p libpna --lib entry::tests::reject_unknown_critical_chunk_in_solid_entry
cargo test -p libpna --lib entry::tests::accept_unknown_ancillary_chunk_in_normal_entry
```

Expected:
- `reject_unknown_critical_chunk_in_normal_entry`: FAIL
- `reject_unknown_critical_chunk_in_solid_entry`: FAIL
- `accept_unknown_ancillary_chunk_in_normal_entry`: PASS (already works)

#### Step 3: Modify NormalEntry parsing

In `lib/src/entry.rs`, line 660, replace:
```rust
_ => extra.push(chunk),
```

With:
```rust
_ => {
    if chunk.ty.is_critical() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown critical chunk type: {}", chunk.ty),
        ));
    }
    extra.push(chunk);
}
```

#### Step 4: Modify SolidEntry parsing

In `lib/src/entry.rs`, line 562, replace:
```rust
_ => extra.push(chunk),
```

With:
```rust
_ => {
    if chunk.ty().is_critical() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown critical chunk type: {}", chunk.ty()),
        ));
    }
    extra.push(chunk);
}
```

Note: SolidEntry uses `chunk.ty()` method, NormalEntry uses `chunk.ty` field directly.

#### Step 5: Run tests to verify they pass

```bash
cargo test -p libpna --lib entry::tests::reject_unknown_critical_chunk
cargo test -p libpna --lib entry::tests::accept_unknown_ancillary_chunk
```

Expected: All PASS

#### Step 6: Run all entry tests

```bash
cargo test -p libpna --lib entry
```

Expected: All tests pass

#### Step 7: Commit

```bash
git add lib/src/entry.rs
git commit -m ":lock: Reject unknown Critical chunks per PNG specification

- NormalEntry: reject unknown Critical chunks in TryFrom<RawEntry>
- SolidEntry: reject unknown Critical chunks in TryFrom<RawEntry>
- Ancillary chunks (lowercase first letter) are still stored in extra"
```

---

## Term 1: libpna Core SPAR Implementation

### Task 1-1: Add SPAR Chunk Type Constant

**Purpose:** ChunkType定数としてSPARを追加する。

**Files:**
- Modify: `lib/src/chunk/types.rs`

#### Step 1: Add SPAR constant

In `lib/src/chunk/types.rs`, after line 105 (`pub const SEND`), add:

```rust
/// Sparse file map
///
/// Contains the logical file size and a list of data regions for sparse files.
/// When present, FDAT contains only the data regions, not the full file content.
pub const SPAR: ChunkType = ChunkType(*b"SPAR");
```

#### Step 2: Update doc comment

In `lib/src/chunk/types.rs`, find the doc comment listing Critical chunks (around line 49-59) and add SPAR:

```rust
/// - **Sparse files**: [`SPAR`](Self::SPAR) (sparse file map)
```

#### Step 3: Add test for SPAR properties

Add to existing test module in `lib/src/chunk/types.rs` (after line 326):

```rust
#[test]
fn spar_chunk_properties() {
    // SPAR: Critical (S=uppercase), Public (P=uppercase),
    //       Reserved (A=uppercase), Unsafe-to-copy (R=uppercase)
    assert!(ChunkType::SPAR.is_critical());
    assert!(!ChunkType::SPAR.is_private());
    assert!(!ChunkType::SPAR.is_set_reserved()); // Reserved bit is NOT set (A is uppercase = normal)
    assert!(!ChunkType::SPAR.is_safe_to_copy());
}
```

#### Step 4: Run tests

```bash
cargo test -p libpna --lib chunk::types
```

Expected: All tests pass

#### Step 5: Commit

```bash
git add lib/src/chunk/types.rs
git commit -m ":sparkles: Add SPAR chunk type for sparse file support"
```

---

### Task 1-2: Create SparseMap Data Structure

**Purpose:** SPARチャンクのデータをパース・シリアライズするための構造体を作成。

**Files:**
- Create: `lib/src/entry/sparse.rs`
- Modify: `lib/src/entry.rs` (add `mod sparse;` and re-export)

#### Step 1: Create sparse.rs module

Create file `lib/src/entry/sparse.rs`:

```rust
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
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
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
        if (data.len() - 8) % 16 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "SPAR chunk has invalid size: {} bytes (expected 8 + n*16)",
                    data.len()
                ),
            ));
        }

        let logical_size = u64::from_be_bytes(data[0..8].try_into().unwrap());
        let entry_count = (data.len() - 8) / 16;
        let mut regions = Vec::with_capacity(entry_count);

        for i in 0..entry_count {
            let base = 8 + i * 16;
            let offset = u64::from_be_bytes(data[base..base + 8].try_into().unwrap());
            let size = u64::from_be_bytes(data[base + 8..base + 16].try_into().unwrap());
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
        if let Some(last) = regions.last() {
            if last.end() > logical_size {
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
        }

        Ok(Self {
            logical_size,
            regions,
        })
    }

    /// Serializes the sparse map to SPAR chunk data.
    ///
    /// The returned bytes can be used as the data portion of a SPAR chunk.
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
```

#### Step 2: Add module declaration to entry.rs

In `lib/src/entry.rs`, after line 8 (`mod reference;`), add:

```rust
mod sparse;
```

In the `pub use self::{ ... }` block (around line 11-19), add the exports:

```rust
pub use self::{
    attr::*,
    builder::{EntryBuilder, SolidEntryBuilder},
    header::*,
    meta::*,
    name::*,
    options::*,
    reference::*,
    sparse::{DataRegion, SparseMap},  // NEW
};
```

#### Step 3: Run tests

```bash
cargo test -p libpna --lib entry::sparse
```

Expected: All tests pass

#### Step 4: Commit

```bash
git add lib/src/entry/sparse.rs lib/src/entry.rs
git commit -m ":sparkles: Add SparseMap data structure for SPAR chunk

- DataRegion: represents a contiguous data region in sparse file
- SparseMap: contains logical_size and list of data regions
- Implements from_bytes/to_bytes for SPAR chunk serialization
- Validates invariants: ascending order, no overlap, within bounds"
```

---

### Task 1-3: Add sparse_map Field to NormalEntry and Parse SPAR

**Purpose:** NormalEntry構造体にsparse_mapフィールドを追加し、SPARチャンクをパースする。

**Files:**
- Modify: `lib/src/entry.rs`

#### Step 1: Add sparse_map field to NormalEntry struct

In `lib/src/entry.rs`, modify `NormalEntry` struct (line 580-587):

```rust
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct NormalEntry<T = Vec<u8>> {
    pub(crate) header: EntryHeader,
    pub(crate) phsf: Option<String>,
    pub(crate) extra: Vec<RawChunk<T>>,
    pub(crate) data: Vec<T>,
    pub(crate) metadata: Metadata,
    pub(crate) xattrs: Vec<ExtendedAttribute>,
    pub(crate) sparse_map: Option<SparseMap>,  // NEW FIELD
}
```

#### Step 2: Add SPAR parsing in TryFrom implementation

In `TryFrom<RawEntry<T>> for NormalEntry<T>` (starts at line 589):

After the existing variable declarations (around line 637), add:
```rust
let mut sparse_map = None;
```

In the match statement (around line 639-661), add before the `_ =>` arm:
```rust
ChunkType::SPAR => {
    if sparse_map.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Duplicate SPAR chunk in entry",
        ));
    }
    sparse_map = Some(SparseMap::from_bytes(chunk.data())?);
}
```

Update the return statement (around line 667-682) to include:
```rust
Ok(Self {
    header,
    phsf,
    extra,
    metadata: Metadata {
        raw_file_size: size,
        compressed_size,
        created: ctime,
        modified: mtime,
        accessed: atime,
        permission,
    },
    data,
    xattrs,
    sparse_map,  // NEW FIELD
})
```

#### Step 3: Add accessor method

In `impl<T> NormalEntry<T>` block (after line 837), add:

```rust
/// Returns the sparse map if this entry represents a sparse file.
///
/// When a sparse map is present:
/// - The entry's FDAT data contains only the data regions (holes are omitted)
/// - Use [`SparseMap::logical_size()`] for the original file size
/// - Use [`SparseMap::regions()`] to determine where each data region belongs
///
/// # Example
///
/// ```rust,ignore
/// if let Some(sparse_map) = entry.sparse_map() {
///     // Create output file with logical size
///     file.set_len(sparse_map.logical_size())?;
///
///     // Write each data region at correct offset
///     let mut reader = entry.reader(options)?;
///     for region in sparse_map.regions() {
///         file.seek(SeekFrom::Start(region.offset()))?;
///         io::copy(&mut reader.take(region.size()), &mut file)?;
///     }
/// }
/// ```
#[inline]
pub fn sparse_map(&self) -> Option<&SparseMap> {
    self.sparse_map.as_ref()
}
```

#### Step 4: Update all From implementations

There are 4 `From` implementations for `NormalEntry` type conversions. Each must include `sparse_map`:

**Line 1032-1043** (`From<NormalEntry<Cow<'a, [u8]>>> for NormalEntry<Vec<u8>>`):
```rust
impl<'a> From<NormalEntry<Cow<'a, [u8]>>> for NormalEntry<Vec<u8>> {
    #[inline]
    fn from(value: NormalEntry<Cow<'a, [u8]>>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            extra: value.extra.into_iter().map(Into::into).collect(),
            data: value.data.into_iter().map(Into::into).collect(),
            metadata: value.metadata,
            xattrs: value.xattrs,
            sparse_map: value.sparse_map,  // NEW
        }
    }
}
```

**Line 1046-1058** (`From<NormalEntry<&'a [u8]>> for NormalEntry<Vec<u8>>`):
```rust
impl<'a> From<NormalEntry<&'a [u8]>> for NormalEntry<Vec<u8>> {
    #[inline]
    fn from(value: NormalEntry<&'a [u8]>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            extra: value.extra.into_iter().map(Into::into).collect(),
            data: value.data.into_iter().map(Into::into).collect(),
            metadata: value.metadata,
            xattrs: value.xattrs,
            sparse_map: value.sparse_map,  // NEW
        }
    }
}
```

**Line 1060-1072** (`From<NormalEntry<Vec<u8>>> for NormalEntry<Cow<'_, [u8]>>`):
```rust
impl From<NormalEntry<Vec<u8>>> for NormalEntry<Cow<'_, [u8]>> {
    #[inline]
    fn from(value: NormalEntry<Vec<u8>>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            extra: value.extra.into_iter().map(Into::into).collect(),
            data: value.data.into_iter().map(Into::into).collect(),
            metadata: value.metadata,
            xattrs: value.xattrs,
            sparse_map: value.sparse_map,  // NEW
        }
    }
}
```

**Line 1074-1086** (`From<NormalEntry<&'a [u8]>> for NormalEntry<Cow<'a, [u8]>>`):
```rust
impl<'a> From<NormalEntry<&'a [u8]>> for NormalEntry<Cow<'a, [u8]>> {
    #[inline]
    fn from(value: NormalEntry<&'a [u8]>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            extra: value.extra.into_iter().map(Into::into).collect(),
            data: value.data.into_iter().map(Into::into).collect(),
            metadata: value.metadata,
            xattrs: value.xattrs,
            sparse_map: value.sparse_map,  // NEW
        }
    }
}
```

#### Step 5: Write tests

Add to test module:

```rust
#[test]
fn parse_entry_with_spar() {
    let spar_data = SparseMap::new(1000, vec![DataRegion::new(0, 100)]).to_bytes();
    let fhed = RawChunk::from_data(ChunkType::FHED, vec![0, 0, 0, 0, 0, 0]);
    let spar = RawChunk::from_data(ChunkType::SPAR, spar_data);
    let fend = RawChunk::from_data(ChunkType::FEND, vec![]);

    let raw_entry = RawEntry(vec![fhed, spar, fend]);
    let entry = NormalEntry::try_from(raw_entry).unwrap();

    let map = entry.sparse_map().expect("sparse_map should be present");
    assert_eq!(map.logical_size(), 1000);
    assert_eq!(map.regions().len(), 1);
    assert_eq!(map.regions()[0].offset(), 0);
    assert_eq!(map.regions()[0].size(), 100);
}

#[test]
fn parse_entry_without_spar() {
    let fhed = RawChunk::from_data(ChunkType::FHED, vec![0, 0, 0, 0, 0, 0]);
    let fdat = RawChunk::from_data(ChunkType::FDAT, vec![1, 2, 3, 4]);
    let fend = RawChunk::from_data(ChunkType::FEND, vec![]);

    let raw_entry = RawEntry(vec![fhed, fdat, fend]);
    let entry = NormalEntry::try_from(raw_entry).unwrap();

    assert!(entry.sparse_map().is_none());
}

#[test]
fn reject_duplicate_spar_chunk() {
    let spar_data = SparseMap::new(1000, vec![]).to_bytes();
    let fhed = RawChunk::from_data(ChunkType::FHED, vec![0, 0, 0, 0, 0, 0]);
    let spar1 = RawChunk::from_data(ChunkType::SPAR, spar_data.clone());
    let spar2 = RawChunk::from_data(ChunkType::SPAR, spar_data);
    let fend = RawChunk::from_data(ChunkType::FEND, vec![]);

    let raw_entry = RawEntry(vec![fhed, spar1, spar2, fend]);
    let result = NormalEntry::try_from(raw_entry);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Duplicate SPAR"));
}

#[test]
fn sparse_map_preserved_in_type_conversion() {
    let spar_data = SparseMap::new(500, vec![DataRegion::new(100, 200)]).to_bytes();
    let fhed = RawChunk::from_data(ChunkType::FHED, vec![0, 0, 0, 0, 0, 0]);
    let spar = RawChunk::from_data(ChunkType::SPAR, spar_data);
    let fend = RawChunk::from_data(ChunkType::FEND, vec![]);

    let raw_entry = RawEntry(vec![fhed, spar, fend]);
    let entry: NormalEntry<Vec<u8>> = NormalEntry::try_from(raw_entry).unwrap();

    // Convert to Cow variant
    let cow_entry: NormalEntry<Cow<[u8]>> = entry.into();
    assert!(cow_entry.sparse_map().is_some());
    assert_eq!(cow_entry.sparse_map().unwrap().logical_size(), 500);
}
```

#### Step 6: Run tests

```bash
cargo test -p libpna --lib entry
```

Expected: All tests pass

#### Step 7: Commit

```bash
git add lib/src/entry.rs
git commit -m ":sparkles: Add SPAR chunk parsing to NormalEntry

- Add sparse_map field to NormalEntry struct
- Parse SPAR chunk in TryFrom<RawEntry>
- Reject duplicate SPAR chunks
- Add sparse_map() accessor method
- Update all From implementations for type conversions"
```

---

### Task 1-4: Add SPAR Writing to NormalEntry

**Purpose:** NormalEntryの書き出し処理にSPARチャンクを追加する。

**Files:**
- Modify: `lib/src/entry.rs`

#### Step 1: Update into_chunks() method

In `impl<T> SealedEntryExt for NormalEntry<T>` (line 753), modify `into_chunks()`:

After line 769 (`vec.extend(self.extra.into_iter().map(Into::into));`), add:

```rust
if let Some(ref sparse_map) = self.sparse_map {
    vec.push(RawChunk::from_data(ChunkType::SPAR, sparse_map.to_bytes()));
}
```

The chunk order will be: FHED → extra → **SPAR** → fSIZ → PHSF → FDAT...

#### Step 2: Update chunks_write_in() method

In `chunks_write_in()` (line 691), after writing extra chunks (line 704-706):

```rust
for ex in &self.extra {
    total += ex.write_chunk_in(writer)?;
}
```

Add:

```rust
if let Some(ref sparse_map) = self.sparse_map {
    total += (ChunkType::SPAR, sparse_map.to_bytes()).write_chunk_in(writer)?;
}
```

#### Step 3: Write tests

Add to test module:

```rust
mod sparse_entry_serialization {
    use super::*;
    use crate::entry::private::SealedEntryExt;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    fn create_sparse_entry() -> NormalEntry<Vec<u8>> {
        NormalEntry {
            header: EntryHeader {
                major: 0,
                minor: 0,
                data_kind: DataKind::File,
                compression: Compression::No,
                encryption: Encryption::No,
                cipher_mode: CipherMode::CBC,
                name: "sparse.bin".into(),
            },
            phsf: None,
            extra: vec![],
            data: vec![vec![0u8; 300]],
            metadata: Metadata::default(),
            xattrs: vec![],
            sparse_map: Some(SparseMap::new(
                1000,
                vec![DataRegion::new(0, 100), DataRegion::new(500, 200)],
            )),
        }
    }

    #[test]
    fn into_chunks_includes_spar() {
        let entry = create_sparse_entry();
        let chunks = entry.into_chunks();

        let spar_chunk = chunks.iter().find(|c| c.ty == ChunkType::SPAR);
        assert!(spar_chunk.is_some(), "SPAR chunk should be present");
    }

    #[test]
    fn round_trip_sparse_entry() {
        let entry = create_sparse_entry();
        let original_sparse_map = entry.sparse_map.clone();

        let chunks = entry.into_chunks();
        let raw_entry = RawEntry(chunks);
        let parsed = NormalEntry::try_from(raw_entry).unwrap();

        assert_eq!(parsed.sparse_map, original_sparse_map);
        assert_eq!(parsed.sparse_map().unwrap().logical_size(), 1000);
        assert_eq!(parsed.sparse_map().unwrap().regions().len(), 2);
    }

    #[test]
    fn no_spar_for_non_sparse_entry() {
        let entry: NormalEntry<Vec<u8>> = NormalEntry {
            header: EntryHeader {
                major: 0,
                minor: 0,
                data_kind: DataKind::File,
                compression: Compression::No,
                encryption: Encryption::No,
                cipher_mode: CipherMode::CBC,
                name: "normal.bin".into(),
            },
            phsf: None,
            extra: vec![],
            data: vec![vec![1, 2, 3, 4]],
            metadata: Metadata::default(),
            xattrs: vec![],
            sparse_map: None,
        };

        let chunks = entry.into_chunks();
        let spar_chunk = chunks.iter().find(|c| c.ty == ChunkType::SPAR);
        assert!(spar_chunk.is_none(), "Non-sparse entry should not have SPAR");
    }
}
```

#### Step 4: Run tests

```bash
cargo test -p libpna --lib entry::tests::sparse_entry_serialization
cargo test -p libpna --lib entry
```

Expected: All tests pass

#### Step 5: Commit

```bash
git add lib/src/entry.rs
git commit -m ":sparkles: Add SPAR chunk writing to NormalEntry

- Emit SPAR chunk in into_chunks() after extra, before fSIZ
- Emit SPAR chunk in chunks_write_in() with same ordering
- Add round-trip tests for sparse entry serialization"
```

---

### Task 1-5: Add sparse_map to EntryBuilder

**Purpose:** EntryBuilderにsparse_map設定機能を追加する。

**Files:**
- Modify: `lib/src/entry/builder.rs`

#### Step 1: Add sparse_map field to EntryBuilder struct

In `lib/src/entry/builder.rs`, modify the `EntryBuilder` struct (line 137-150):

```rust
pub struct EntryBuilder {
    header: EntryHeader,
    phsf: Option<String>,
    iv: Option<Vec<u8>>,
    data: Option<CompressionWriter<CipherWriter<FlattenWriter>>>,
    created: Option<Duration>,
    last_modified: Option<Duration>,
    accessed: Option<Duration>,
    permission: Option<Permission>,
    store_file_size: bool,
    file_size: u128,
    xattrs: Vec<ExtendedAttribute>,
    extra_chunks: Vec<RawChunk>,
    sparse_map: Option<SparseMap>,  // NEW FIELD
}
```

#### Step 2: Initialize in new() constructor

In the `new()` const fn (line 153-168), add field initialization:

```rust
const fn new(header: EntryHeader) -> Self {
    Self {
        header,
        phsf: None,
        iv: None,
        data: None,
        created: None,
        last_modified: None,
        accessed: None,
        permission: None,
        store_file_size: true,
        file_size: 0,
        xattrs: Vec::new(),
        extra_chunks: Vec::new(),
        sparse_map: None,  // NEW
    }
}
```

#### Step 3: Add setter method

Add after the existing setter methods (around line 400):

```rust
/// Sets the sparse map for this entry.
///
/// When a sparse map is set, the entry will include a SPAR chunk.
/// The caller is responsible for writing only the data regions, not the full file content.
///
/// # Important
///
/// The total bytes written via [`Write`] must equal [`SparseMap::data_size()`],
/// not [`SparseMap::logical_size()`]. The sparse map describes where each written
/// byte belongs in the logical file space.
///
/// # Example
///
/// ```rust
/// # use std::io::{self, Write, Seek, SeekFrom};
/// use libpna::{EntryBuilder, WriteOptions, SparseMap, DataRegion};
///
/// # fn main() -> io::Result<()> {
/// // File has 1000 bytes logically, but only 300 bytes of data
/// let sparse_map = SparseMap::new(1000, vec![
///     DataRegion::new(0, 100),    // First 100 bytes
///     DataRegion::new(500, 200),  // 200 bytes starting at offset 500
/// ]);
///
/// let mut builder = EntryBuilder::new_file("sparse.bin".into(), WriteOptions::store())?;
/// builder.set_sparse_map(sparse_map);
///
/// // Write exactly 300 bytes (the data regions, in order)
/// builder.write_all(&[0u8; 100])?;  // Region 1
/// builder.write_all(&[1u8; 200])?;  // Region 2
///
/// let entry = builder.build()?;
/// assert!(entry.sparse_map().is_some());
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn set_sparse_map(&mut self, sparse_map: SparseMap) -> &mut Self {
    self.sparse_map = Some(sparse_map);
    self
}
```

#### Step 4: Add validation and pass sparse_map to NormalEntry in build()

In `build()` method (line 473-501), add validation before constructing NormalEntry:

```rust
// Validate sparse_map data size matches written data
if let Some(ref sparse_map) = self.sparse_map {
    let expected = sparse_map.data_size() as u128;
    if self.file_size != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Sparse map data_size ({}) does not match written bytes ({})",
                expected, self.file_size
            ),
        ));
    }
}

Ok(NormalEntry {
    header: self.header,
    phsf: self.phsf,
    extra: self.extra_chunks,
    data,
    metadata,
    xattrs: self.xattrs,
    sparse_map: self.sparse_map,  // NEW
})
```

#### Step 5: Add use statement if needed

At the top of `lib/src/entry/builder.rs`, ensure `SparseMap` is imported:

```rust
use crate::entry::{
    // ... existing imports ...
    sparse::SparseMap,  // Add if not already visible
};
```

Note: Since `SparseMap` is re-exported from `entry`, it may already be available through the existing imports.

#### Step 6: Write tests

Add to `lib/src/entry/builder.rs` or create a test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::sparse::DataRegion;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn entry_builder_with_sparse_map() -> io::Result<()> {
        let sparse_map = SparseMap::new(
            1000,
            vec![DataRegion::new(0, 100), DataRegion::new(500, 200)],
        );

        let mut builder = EntryBuilder::new_file("sparse.bin".into(), WriteOptions::store())?;
        builder.set_sparse_map(sparse_map);
        builder.write_all(&[0u8; 300])?; // Write data regions
        let entry = builder.build()?;

        let map = entry.sparse_map().expect("sparse_map should be present");
        assert_eq!(map.logical_size(), 1000);
        assert_eq!(map.data_size(), 300);
        assert_eq!(map.regions().len(), 2);
        Ok(())
    }

    #[test]
    fn entry_builder_without_sparse_map() -> io::Result<()> {
        let mut builder = EntryBuilder::new_file("normal.bin".into(), WriteOptions::store())?;
        builder.write_all(&[1, 2, 3, 4])?;
        let entry = builder.build()?;

        assert!(entry.sparse_map().is_none());
        Ok(())
    }

    #[test]
    fn entry_builder_all_hole_sparse() -> io::Result<()> {
        // File is 1GB logically but contains no data
        let sparse_map = SparseMap::new(1024 * 1024 * 1024, vec![]);

        let mut builder = EntryBuilder::new_file("hole.bin".into(), WriteOptions::store())?;
        builder.set_sparse_map(sparse_map);
        // Don't write any data
        let entry = builder.build()?;

        let map = entry.sparse_map().expect("sparse_map should be present");
        assert!(map.is_all_hole());
        assert_eq!(map.logical_size(), 1024 * 1024 * 1024);
        assert_eq!(map.data_size(), 0);
        Ok(())
    }

    #[test]
    fn sparse_map_only_applies_to_files() -> io::Result<()> {
        // Directories don't use sparse maps (they have no data)
        let dir = EntryBuilder::new_dir("my_dir/".into());
        let entry = dir.build()?;
        assert!(entry.sparse_map().is_none());
        Ok(())
    }
}
```

#### Step 7: Run tests

```bash
cargo test -p libpna --lib entry::builder
```

Expected: All tests pass

#### Step 8: Commit

```bash
git add lib/src/entry/builder.rs
git commit -m ":sparkles: Add sparse map support to EntryBuilder

- Add sparse_map field to EntryBuilder struct
- Add set_sparse_map() setter method with documentation
- Pass sparse_map to NormalEntry in build()"
```

---

### Task 1-6: Export SparseMap from libpna Public API

**Purpose:** SparseMap, DataRegionをlibpnaの公開APIとしてエクスポートし、ドキュメントを確認する。

**Files:**
- Modify: `lib/src/lib.rs`

#### Step 1: Verify exports

Check that `pub use entry::*;` (line 150) already exports `SparseMap` and `DataRegion` since they are re-exported from `entry.rs`.

If not visible, add explicit re-exports in `lib/src/lib.rs`:

```rust
pub use entry::{DataRegion, SparseMap};
```

#### Step 2: Update lib.rs documentation

In the Key Types section of the doc comment (around line 112-120), add:

```rust
//! - [`SparseMap`] / [`DataRegion`] - Sparse file representation
```

#### Step 3: Verify public API

```bash
cargo doc -p libpna --no-deps
# Check that SparseMap and DataRegion appear in the docs
```

#### Step 4: Run full test suite

```bash
cargo test -p libpna --all-features
```

Expected: All tests pass

#### Step 5: Commit

```bash
git add lib/src/lib.rs
git commit -m ":memo: Export SparseMap and DataRegion from public API

- Ensure SparseMap and DataRegion are visible in public API
- Add to Key Types documentation"
```

---

## Term 2: CLI Integration

### Design Decisions

| 項目 | 決定 |
|------|------|
| スパース検出方法 | SEEK_HOLE/SEEK_DATA 優先、非対応なら st_blocks フォールバック |
| st_blocks フォールバック時 | ホール位置不明なら SPAR なし（通常ファイル扱い） |
| Windows 対応 | 今回はスキップ、将来対応 |
| 作成時オプション (native CLI) | `--sparse` / `--no-sparse`（デフォルト無効） |
| 作成時オプション (stdio) | `--sparse` / `-S` / `--no-sparse` (bsdtar 互換) |
| 展開時の動作 | オプション不要、SPAR があれば常に復元 |
| 展開方法 | lseek + write（ポータブル） |
| テスト方法 | libpna レベル（基本）+ ファイルシステム依存（分離） |

---

### Task 2-1: Add Sparse Detection Utility

**Purpose:** ファイルのスパース領域を検出するユーティリティを作成。

**Files:**
- Create: `cli/src/utils/sparse.rs`
- Modify: `cli/src/utils/mod.rs`

#### Step 1: Create sparse.rs module

```rust
//! Sparse file detection utilities.

use libpna::{DataRegion, SparseMap};
use std::fs::File;
use std::io;

/// Detects sparse regions in a file.
///
/// Returns `Some(SparseMap)` if the file is sparse, `None` otherwise.
///
/// Detection strategy:
/// 1. Try SEEK_HOLE/SEEK_DATA (Linux, macOS, FreeBSD)
/// 2. If unsupported, check st_blocks vs file size
/// 3. If st_blocks indicates sparse but holes not detectable, return None
#[cfg(unix)]
pub fn detect_sparse_map(file: &File) -> io::Result<Option<SparseMap>> {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::io::AsRawFd;

    let metadata = file.metadata()?;
    let file_size = metadata.len();

    if file_size == 0 {
        return Ok(None);
    }

    // Try SEEK_HOLE/SEEK_DATA first
    let fd = file.as_raw_fd();
    match detect_with_seek_hole_data(fd, file_size) {
        Ok(Some(map)) => return Ok(Some(map)),
        Ok(None) => return Ok(None), // Not sparse
        Err(e) if is_seek_hole_unsupported(&e) => {
            // Fall through to st_blocks check
        }
        Err(e) => return Err(e),
    }

    // Fallback: check st_blocks
    // If blocks * 512 >= size, file is not sparse
    let block_bytes = metadata.blocks() * 512;
    if block_bytes >= file_size {
        return Ok(None);
    }

    // File appears sparse by st_blocks, but we can't determine hole positions
    // Return None to treat as normal file
    Ok(None)
}

#[cfg(unix)]
fn detect_with_seek_hole_data(fd: std::os::unix::io::RawFd, file_size: u64) -> io::Result<Option<SparseMap>> {
    let mut regions = Vec::new();
    let mut pos: i64 = 0;

    loop {
        // Find next data region
        let data_start = unsafe { libc::lseek(fd, pos, libc::SEEK_DATA) };
        if data_start < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ENXIO) {
                // No more data - rest is hole
                break;
            }
            return Err(err);
        }

        // Find end of data region (next hole)
        let hole_start = unsafe { libc::lseek(fd, data_start, libc::SEEK_HOLE) };
        if hole_start < 0 {
            return Err(io::Error::last_os_error());
        }

        let data_size = (hole_start - data_start) as u64;
        if data_size > 0 {
            regions.push(DataRegion::new(data_start as u64, data_size));
        }

        pos = hole_start;
        if pos as u64 >= file_size {
            break;
        }
    }

    // Restore file position
    unsafe { libc::lseek(fd, 0, libc::SEEK_SET) };

    // Determine if file is actually sparse
    if regions.is_empty() && file_size > 0 {
        // Entire file is a hole
        Ok(Some(SparseMap::new(file_size, vec![])))
    } else if regions.len() == 1 && regions[0].offset() == 0 && regions[0].size() == file_size {
        // File is not sparse (single region covering entire file)
        Ok(None)
    } else {
        Ok(Some(SparseMap::new(file_size, regions)))
    }
}

#[cfg(unix)]
fn is_seek_hole_unsupported(err: &io::Error) -> bool {
    matches!(err.raw_os_error(), Some(libc::EOPNOTSUPP) | Some(libc::EINVAL))
}

#[cfg(not(unix))]
pub fn detect_sparse_map(_file: &File) -> io::Result<Option<SparseMap>> {
    // Windows: sparse detection not implemented yet
    Ok(None)
}
```

#### Step 2: Add module to utils/mod.rs

```rust
pub(crate) mod sparse;
```

#### Step 3: Write tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn detect_non_sparse_file() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let result = detect_sparse_map(file.as_file()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn detect_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let result = detect_sparse_map(file.as_file()).unwrap();
        assert!(result.is_none());
    }
}
```

#### Step 4: Run tests

```bash
cargo test -p portable-network-archive --lib utils::sparse
```

#### Step 5: Commit

```bash
git add cli/src/utils/sparse.rs cli/src/utils/mod.rs
git commit -m ":sparkles: Add sparse file detection utility

- SEEK_HOLE/SEEK_DATA for accurate detection (Unix)
- st_blocks fallback check
- Windows: not implemented (returns None)"
```

---

### Task 2-2: Add --sparse Option to Create Command

**Purpose:** アーカイブ作成時にスパース検出を有効にするオプションを追加。

**Files:**
- Modify: `cli/src/command/create.rs` (add --sparse/--no-sparse options)
- Modify: `cli/src/command/commons.rs` or args definition
- Modify: `cli/src/command/core.rs` (integrate detection)

#### Step 1: Add CLI options

In the create command arguments:

```rust
/// Detect and preserve sparse file structure
#[arg(long)]
sparse: bool,

/// Do not detect sparse files (default)
#[arg(long, conflicts_with = "sparse")]
no_sparse: bool,
```

#### Step 2: Pass option to entry creation

When creating entries, if `--sparse` is enabled and the source is a regular file:

```rust
if args.sparse {
    if let Some(sparse_map) = crate::utils::sparse::detect_sparse_map(&file)? {
        entry_builder.set_sparse_map(sparse_map);
        // Read only data regions when writing
    }
}
```

#### Step 3: Modify file reading for sparse files

When sparse_map is set, read and write only the data regions:

```rust
if let Some(ref sparse_map) = sparse_map {
    for region in sparse_map.regions() {
        file.seek(SeekFrom::Start(region.offset()))?;
        let mut take = (&mut file).take(region.size());
        io::copy(&mut take, &mut entry_builder)?;
    }
} else {
    io::copy(&mut file, &mut entry_builder)?;
}
```

#### Step 4: Add to stdio subcommand with short option

In stdio args, add `-S` short option:

```rust
/// Detect and preserve sparse file structure
#[arg(long, short = 'S')]
sparse: bool,
```

#### Step 5: Commit

```bash
git add cli/src/command/create.rs cli/src/command/commons.rs cli/src/command/core.rs
git commit -m ":sparkles: Add --sparse option to create command

- --sparse: enable sparse file detection
- --no-sparse: explicitly disable (default)
- stdio: -S short option for bsdtar compatibility"
```

---

### Task 2-3: Implement Sparse Restoration in Extract Command

**Purpose:** SPAR付きエントリを展開時にスパース構造を復元する。

**Files:**
- Modify: `cli/src/command/extract.rs`

#### Step 1: Detect SPAR in entry

When extracting a file entry:

```rust
if let Some(sparse_map) = entry.sparse_map() {
    extract_sparse_file(entry, sparse_map, &output_path, options)?;
} else {
    // Normal extraction
    extract_normal_file(entry, &output_path, options)?;
}
```

#### Step 2: Implement sparse extraction with validation

```rust
fn extract_sparse_file<R: Read>(
    mut reader: R,
    sparse_map: &SparseMap,
    path: &Path,
    options: &ExtractOptions,
) -> io::Result<()> {
    let mut file = File::create(path)?;

    // Set file to logical size (creates hole at end if needed)
    file.set_len(sparse_map.logical_size())?;

    // Write each data region at correct offset
    for region in sparse_map.regions() {
        file.seek(SeekFrom::Start(region.offset()))?;

        // Read exactly region.size() bytes, error if insufficient
        let mut buf = vec![0u8; 8192];
        let mut remaining = region.size();
        while remaining > 0 {
            let to_read = std::cmp::min(remaining as usize, buf.len());
            let n = reader.read(&mut buf[..to_read])?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!(
                        "Sparse region at offset {} expects {} bytes, but data ended {} bytes short",
                        region.offset(),
                        region.size(),
                        remaining
                    ),
                ));
            }
            file.write_all(&buf[..n])?;
            remaining -= n as u64;
        }
    }

    Ok(())
}
```

**Note on Windows**: On Windows, `set_len()` + `seek()` + `write()` may not create actual sparse holes. The file will be logically correct but may consume full disk space. Future enhancement: call `DeviceIoControl(FSCTL_SET_SPARSE)` before writing to enable sparse file support on NTFS.
```

#### Step 3: Commit

```bash
git add cli/src/command/extract.rs
git commit -m ":sparkles: Implement sparse file restoration on extract

- Detect SPAR chunk in entries
- Use lseek + write for portable hole creation
- No CLI option needed (always restore if SPAR present)"
```

---

### Task 2-4: Add CLI Integration Tests

**Purpose:** スパースファイルのラウンドトリップテストを追加。

**Files:**
- Create: `cli/tests/cli/sparse.rs`
- Modify: `cli/tests/cli/mod.rs`

#### Step 1: Create sparse.rs test module

```rust
//! Sparse file integration tests.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Test: Archive with --sparse option includes SPAR chunk for sparse files.
///
/// Precondition: libpna-level SparseMap can be set on entries.
/// Action: Create archive with --sparse, then list/inspect.
/// Expectation: Archive contains entries with sparse metadata.
#[test]
fn create_with_sparse_option() {
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("test.pna");

    // Create a file and set sparse map via libpna
    // (This tests the libpna integration, not filesystem sparse detection)
    // ...
}

/// Test: Extract restores sparse file structure.
///
/// Precondition: Archive contains entry with SPAR chunk.
/// Action: Extract the archive.
/// Expectation: Extracted file has correct logical size.
#[test]
fn extract_sparse_entry() {
    // ...
}

/// Test: --no-sparse disables sparse detection.
#[test]
fn create_with_no_sparse_option() {
    // ...
}
```

#### Step 2: Add filesystem-dependent tests (optional)

```rust
/// Filesystem-dependent sparse file tests.
///
/// These tests require a filesystem that supports SEEK_HOLE/SEEK_DATA.
/// Skip on tmpfs or other non-supporting filesystems.
#[cfg(all(unix, feature = "sparse-fs-test"))]
mod filesystem_tests {
    // Actual sparse file creation and detection tests
}
```

#### Step 3: Add module to mod.rs

```rust
mod sparse;
```

#### Step 4: Run tests

```bash
cargo test -p portable-network-archive --test cli sparse
```

#### Step 5: Commit

```bash
git add cli/tests/cli/sparse.rs cli/tests/cli/mod.rs
git commit -m ":white_check_mark: Add sparse file integration tests

- libpna-level tests (filesystem-independent)
- Optional filesystem-dependent tests behind feature flag"
```

---

## Design Decisions Summary

### Term 0/1 (libpna)

| 項目 | 決定 |
|------|------|
| SparseMap::new() 検証 | debug_asserts のみ（リリースでは検証なし） |
| チャンク順序テスト | 不要（順序非依存の設計を維持） |
| SparseMap の Clone | derive するが明示的には使わない |
| SolidEntry 内の SPAR | NormalEntry 内のみ有効 |
| set_sparse_map 制限 | なし（全エントリ種別で設定可能） |
| SPAR 書き出し条件 | sparse_map があれば常に書き出す |
| SPAR パース時の種別チェック | なし（常にパースして格納） |
| DataRegion フィールド | private、アクセサ経由 |

### Term 2 (CLI)

| 項目 | 決定 |
|------|------|
| スパース検出方法 | SEEK_HOLE/SEEK_DATA 優先、非対応なら st_blocks フォールバック |
| st_blocks フォールバック時 | ホール位置不明なら SPAR なし（通常ファイル扱い） |
| Windows 対応 | 今回はスキップ、将来対応 |
| 作成時オプション (native CLI) | `--sparse` / `--no-sparse`（デフォルト無効） |
| 作成時オプション (stdio) | `--sparse` / `-S` / `--no-sparse` (bsdtar 互換) |
| 展開時の動作 | オプション不要、SPAR があれば常に復元 |
| 展開方法 | lseek + write（ポータブル） |
| テスト方法 | libpna レベル（基本）+ ファイルシステム依存（分離） |

---

## Summary

| Term | Tasks | Description |
|------|-------|-------------|
| 0 | 0-1 | Prerequisites: Reject unknown Critical chunks in NormalEntry and SolidEntry |
| 1 | 1-1 to 1-6 | libpna core: SPAR chunk type, SparseMap structure, parsing, writing, builder, exports |
| 2 | 2-1 to 2-4 | CLI: Detection, creation, extraction, testing |

**Dependencies:**
- Term 0 must complete before Term 1 (unknown Critical chunk rejection required for backward compatibility safety)
- Term 1 must complete before Term 2 (CLI depends on libpna SparseMap API)
- Within each Term, tasks are sequential

**Key Code Locations:**

| Component | File | Lines |
|-----------|------|-------|
| ChunkType::SPAR | `lib/src/chunk/types.rs` | After line 105 |
| SparseMap | `lib/src/entry/sparse.rs` | New file |
| NormalEntry.sparse_map | `lib/src/entry.rs` | Line 580-587, 660, 667-682, 758-826, 1032-1086 |
| EntryBuilder.sparse_map | `lib/src/entry/builder.rs` | Line 137-150, 153-168, 473-501 |
| SolidEntry Critical rejection | `lib/src/entry.rs` | Line 562 |
