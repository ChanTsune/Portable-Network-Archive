## 2025-05-14 - Optimized Chunk Writing with Single-Pass CRC Calculation

**Learning:** Blanket optimization of `ChunkExt::write_chunk_in` to use a single-pass CRC calculation can cause a performance regression for `RawChunk` because `RawChunk` already has a cached/pre-calculated CRC. Re-calculating it during write is redundant.

**Action:** Use a specialized single-pass utility (`write_chunk_single_pass_in`) only for ephemeral/tuple-based chunks (like `(ChunkType::FDAT, data)`) that don't have a pre-calculated CRC. This preserves the performance of `RawChunk` while optimizing hot paths for data and metadata chunks.
