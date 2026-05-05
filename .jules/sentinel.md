## 2026-05-05 - Harden archive part number validation
**Vulnerability:** Potential panic in multi-part archive handling due to integer overflow.
**Learning:** Malformed archives or extremely large split sets could trigger panics when incrementing part numbers (e.g., `archive_number + 1`). While `u32::MAX` parts are unlikely, safe arithmetic is required for robustness against untrusted input.
**Prevention:** Use `checked_add` for all counters derived from archive headers or used in loop increments that determine archive structure.
