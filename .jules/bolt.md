# Bolt's Journal

## 2025-05-15 - FlattenWriter Allocation Bottleneck
**Learning:** In scenarios with many small writes, the previous `FlattenWriter` implementation created a new `Vec` for every `write` call, leading to excessive allocations and fragmentation. Reusing the last chunk's remaining capacity reduced allocations and improved write performance by ~64% in a micro-benchmark.
**Action:** When implementing buffering or flattening writers, always check if the current buffer can accommodate more data before allocating a new one. This is especially critical for hot paths that might receive small data increments.
