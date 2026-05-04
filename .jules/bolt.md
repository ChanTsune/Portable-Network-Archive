## 2026-05-04 - Eliminate heap allocations in StreamCipherWriter::write

**Learning:** `StreamCipherWriter::write` was using `buf.to_vec()` to create a heap-allocated copy of the input buffer before encryption. This causes unnecessary $O(N)$ allocations and copies for every write operation.

**Action:** Use `apply_keystream_b2b` with a fixed-size stack buffer (e.g., 4KB) to process input in chunks. This reduces memory overhead to $O(1)$ and improved AES-CTR write performance by ~7.6%.
