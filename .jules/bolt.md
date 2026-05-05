## 2024-05-23 - Argon2 Overhead in Encryption Benchmarks
**Learning:** Argon2 key derivation has a high constant cost that masks micro-optimizations in the encryption pipeline when using small payloads.
**Action:** Use larger payloads or separate key derivation when benchmarking encryption micro-optimizations.
