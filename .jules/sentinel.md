## 2025-02-12 - Bound Link Target Size
**Vulnerability:** Denial of Service (DoS) via memory exhaustion when reading excessively large symbolic or hard link targets into memory.
**Learning:** Link targets are stored in the archive and need to be decompressed and read into memory for processing (extraction, listing, or comparison). Without a size limit, a malicious archive could contain a "link target" that is gigabytes in size, causing the archiver to run out of memory.
**Prevention:** Enforce a reasonable maximum size (`MAX_LINK_TARGET_SIZE`) using `std::io::Read::take` when reading link targets or any other metadata that is not naturally bounded by the format or operating system limits.
