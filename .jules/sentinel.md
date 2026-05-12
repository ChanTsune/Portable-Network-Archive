## 2025-05-14 - Arithmetic Hardening in Entry Parsing and Splitting
**Vulnerability:** Potential panics from integer overflows during archive entry parsing and multi-part archive splitting.
**Learning:** Cumulative size calculations (like `compressed_size` or total entry length) and counters (like archive part numbers) in archive logic are vulnerable to overflows when processing large or malformed archives, especially on 32-bit platforms where `usize::MAX` is smaller.
**Prevention:** Use `saturating_add` for size and length calculations to safely maintain upper bounds without crashing, and `checked_add` for structural fields like sequence numbers where an explicit error is preferable to wrap-around or saturation.
