## 2025-05-24 - Oversized Link Target DoS
**Vulnerability:** Memory exhaustion (DoS) during archive extraction. Symbolic and hard link targets were read into memory using `io::read_to_string` without size limits. A malicious archive entry could specify a multi-gigabyte target path, causing an OOM crash.
**Learning:** Metadata-like fields (link targets, names, attributes) are often assumed to be "small" and read entirely into memory. Archives are untrusted and can violate these assumptions.
**Prevention:** Always use `io::Read::take` to bound memory allocations when reading untrusted variable-length fields from an archive, even for fields that "should" be small like file paths.
