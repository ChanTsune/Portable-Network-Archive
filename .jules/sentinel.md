## 2025-05-15 - Integer overflows in archive entry size and length calculations
**Vulnerability:** Potential integer overflows in entry parsing (`NormalEntry::try_from`), writing (`chunks_write_in`), and CLI-side archive splitting logic.
**Learning:** Untrusted data from archive headers (like chunk lengths) was being added using wrapping or saturating arithmetic (`+=`, `.sum()`), which could lead to inconsistent state or logic errors if values exceeded `usize::MAX`.
**Prevention:** Always use `checked_add` or `checked_mul` when accumulating lengths or sizes derived from untrusted archive input, and propagate an `io::Error` on overflow to fail securely.
