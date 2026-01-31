# Sparse File Support Design

This document specifies the design for adding sparse file support to the PNA (Portable Network Archive) format.

## Overview

Sparse files are files containing "holes" - regions that have never been written and read as zeros. On many operating systems, actual disk storage is not allocated for holes, but they are counted in the logical file length.

This extension enables PNA to:
1. **Storage efficiency**: Omit hole regions from the archive, reducing archive size
2. **Complete restoration**: Reconstruct sparse files with their original sparse structure on extraction

## Design Principles

- Follow PNA's streaming-first design
- Reuse existing chunk infrastructure (FDAT for data, new chunk for sparse map)
- Keep implementation details out of specification (hole detection methods, restoration strategies)

## Prerequisites

This design requires libpna to properly handle unknown Critical chunks per PNG specification:

- **Unknown Critical chunks MUST cause an error** (not silently stored in `extra`)
- This ensures old readers fail safely rather than producing corrupt output
- Implementation: Add validation in entry parsing to reject unknown Critical chunk types

## SPAR Chunk Specification

### Chunk Type

```
Type: SPAR (0x53 0x50 0x41 0x52)
```

**Naming convention (PNG-derived):**
- S (uppercase): Critical - must be understood to process the entry
- P (uppercase): Public - defined in the official specification
- A (uppercase): Reserved - must be uppercase per spec
- R (uppercase): Unsafe-to-copy - must be recalculated if entry data is modified

### Chunk Data Format

```
+------------------+------------------+------------------+-----+
| logical_size     | offset_0         | size_0           | ... |
| (8 bytes, u64)   | (8 bytes, u64)   | (8 bytes, u64)   |     |
+------------------+------------------+------------------+-----+
```

All values are unsigned 64-bit integers in **big-endian** byte order.

| Field | Type | Description |
|-------|------|-------------|
| logical_size | u64 | Original file size after restoration (includes holes) |
| offset_n | u64 | Byte offset of data region n in the logical file |
| size_n | u64 | Size in bytes of data region n |

**Entry count**: `(chunk_data_length - 8) / 16`

### Entry Constraints

1. **Ascending order**: `entries[i].offset < entries[i+1].offset`
2. **Non-overlapping**: `entries[i].offset + entries[i].size <= entries[i+1].offset`
3. **Within bounds**: `offset + size <= logical_size` for all entries

**Allowed edge cases:**
- Zero entries (entire file is a hole)
- Size-zero entries (implementations may ignore)

**Restrictions:**
- One SPAR chunk per entry

### Placement Within Entry

SPAR MUST appear:
- After `FHED`
- Before the first `FDAT` chunk

This enables streaming extraction - readers know the sparse map before receiving data.

```
FHED
  [fSIZ]          (optional, ancillary)
  [SPAR]          (required for sparse files)
  [PHSF]          (optional, for encrypted entries)
  [FDAT...]       (data regions concatenated)
  [metadata...]   (cTIM, mTIM, fPRM, xATR, etc.)
FEND
```

### Applicable Entry Types

SPAR applies only to regular files (`kind = 0` in FHED).

| Entry Kind | SPAR Applicable |
|------------|-----------------|
| File (0) | Yes |
| Directory (1) | No |
| SymbolicLink (2) | No |
| HardLink (3) | No |

### Relationship with FDAT

When SPAR is present:
- FDAT contains **only data regions** (holes omitted)
- Data regions are concatenated in offset order
- Total FDAT data length equals sum of all `size_n` values in SPAR
- **FDAT may be absent** if there are zero data regions (entire file is a hole)

**Offset semantics**: SPAR offsets refer to positions in the **logical file space** (after decompression/decryption), not positions within the compressed/encrypted FDAT data.

```
Example:
  Logical file: [DATA 0-100][HOLE 100-300][DATA 300-400]
  SPAR: logical_size=400, entries=[(0,100), (300,100)]
  FDAT: 200 bytes (100 + 100, before compression)
```

### Relationship with fSIZ

- SPAR contains its own `logical_size` field
- fSIZ is ancillary (a hint, not authoritative) - implementations SHOULD NOT trust it
- Readers MUST use SPAR's `logical_size` for sparse file restoration
- fSIZ may be present or absent; its value is independent of SPAR

### Solid Mode Support

SPAR is permitted within entries inside SDAT (Solid mode).

```
SHED
  [PHSF]
  [SDAT...]  ─┬─ Contains compressed/encrypted:
              │    FHED
              │      [SPAR]      <- Allowed
              │      [FDAT...]
              │    FEND
              │    FHED
              │      ...
              │    FEND
SEND
```

## Reading Algorithm

```
1. Read FHED
2. Read chunks until FEND:
   - If SPAR: parse sparse map, store logical_size and entries
   - If FDAT: accumulate data
   - Other chunks: process as normal
3. If SPAR was present:
   a. Create output file with logical_size (or use seek)
   b. For each entry in SPAR (in order):
      - Seek to entry.offset
      - Write entry.size bytes from accumulated FDAT data
   c. Result: sparse file with holes between data regions
4. If SPAR was absent:
   a. Write FDAT data directly (normal file)
```

## Writing Algorithm

```
1. Detect sparse regions in source file (implementation-defined)
2. Build sparse map: list of (offset, size) for data regions
3. Write FHED
4. Write SPAR chunk:
   - logical_size = original file size
   - entries = sparse map (ascending offset order)
5. Write FDAT chunks:
   - Read and write only data regions (skip holes)
6. Write metadata chunks (cTIM, mTIM, fPRM, etc.)
7. Write FEND
```

## Implementation Notes

### Hole Detection (Implementation-Defined)

Implementations may use any method to detect holes:
- `lseek(SEEK_HOLE/SEEK_DATA)` - efficient, filesystem-aware
- `FIEMAP` ioctl - Linux-specific, detailed extent info
- Byte scanning - portable but slow
- Heuristics - block-aligned zero detection

### Restoration Strategies (Implementation-Defined)

Implementations may restore sparse files using:
- `lseek` + `write` - portable, creates holes on supporting filesystems
- `fallocate(FALLOC_FL_PUNCH_HOLE)` - explicit hole creation
- Direct write - fallback, no sparse optimization

### Compression Interaction

- Holes (zeros) compress well, but sparse representation avoids compression overhead entirely
- For highly sparse files, SPAR provides better efficiency than relying on compression alone
- Solid mode benefits less from SPAR (compression context shared across entries)

## Backward Compatibility

### Existing Readers

**After libpna is updated to reject unknown Critical chunks** (see Prerequisites), readers that do not understand SPAR will:
1. Encounter an unknown Critical chunk (SPAR)
2. Return an error for the entry
3. **Cannot** extract the file (safe failure, no data corruption)

This is intentional - without SPAR, data placement is undefined and extraction would produce corrupt output.

**Note**: This requires updating libpna to follow PNG specification for Critical chunk handling. Current implementation silently stores unknown chunks, which would lead to data corruption.

### Future Extensions

If larger offsets become necessary:
- Define new chunk type (e.g., `SPA2`) with u128 fields
- Existing SPAR remains valid for files within u64 range

## Examples

### Example 1: Simple Sparse File

```
File: 1 MB with 100 KB data at start, rest is hole
Logical size: 1,048,576 bytes
Data region: offset=0, size=102,400

SPAR chunk data (hex, big-endian):
  00 00 00 00 00 10 00 00  (logical_size = 1048576)
  00 00 00 00 00 00 00 00  (offset = 0)
  00 00 00 00 00 01 90 00  (size = 102400)

Total: 24 bytes
```

### Example 2: Multiple Data Regions

```
File: 1 MB with data at [0-100KB], [500KB-600KB], [900KB-1MB]
Logical size: 1,048,576 bytes

SPAR entries:
  (0, 102400)
  (512000, 102400)
  (921600, 126976)

SPAR chunk data: 8 + (3 * 16) = 56 bytes
FDAT total: 102400 + 102400 + 126976 = 331,776 bytes
Savings: 1,048,576 - 331,776 = 716,800 bytes (68% reduction)
```

### Example 3: Entire File is Hole

```
File: 1 GB sparse file with no data
Logical size: 1,073,741,824 bytes
Data regions: none

SPAR chunk data (hex):
  00 00 00 00 40 00 00 00  (logical_size = 1073741824)

Total: 8 bytes
FDAT: 0 bytes
```

## References

- [GNU tar Sparse Formats](https://www.gnu.org/software/tar/manual/html_section/Sparse-Formats.html)
- [libarchive Sparse File Handling](https://github.com/libarchive/libarchive)
- [SEEK_HOLE/SEEK_DATA](https://man7.org/linux/man-pages/man2/lseek.2.html)

## Changelog

- 2026-01-31: Initial design document
- 2026-01-31: Added specification clarifications after review
  - Added prerequisite: libpna must reject unknown Critical chunks
  - Clarified: only one SPAR chunk per entry allowed
  - Clarified: FDAT may be absent for all-hole files
  - Clarified: SPAR offsets are in logical file space (post-decompression)
