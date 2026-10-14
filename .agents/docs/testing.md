# Test Design Rules

## Regression Value & Layer Boundaries

Search for tests of the same fault class before adding a case. Cipher round-trips covered by `lib/tests/extract_compatibility.rs` do not justify equivalent CLI tests.

A body with no assertion beyond `.unwrap()` / `.assert().success()` fails the gate. Accepting a flag alone tests clap rather than this project's CLI behavior.

Re-checking an already pinned constant adds no value: if `X::from_byte(2) == GCM` and `GCM.to_byte() == 2` are tested, a third round-trip test is redundant.

## Oracles & Preconditions

- Use `is_ok()`, `is_some()`, `contains(..)`, or `.success()` only when the coarse property is the whole contract.
- For specified layouts (chunk framing, FHED/SHED/xATR fields, PHSF), assert literal expected bytes rather than encode→decode identity; field reordering can survive a round-trip. See `lib/src/archive/header.rs` and `lib/src/io.rs` for examples.
- Do not calculate expected values using the expression under test, such as `assert_eq!(f(x), f(x))`, or by re-deriving an HKDF.
- No bare `return;` skips or conditional assertions such as `if let Ok(..) { assert!(..) }`. CLI runtime capability handling is defined in [CLI testing details](cli-integration-testing.md#runtime-capabilities).

## Closed Sets

For sets such as every `Compression`, cipher, or `BoundedString`/`BoundedBytes` variant, use these shapes in order of preference:

1. Independently readable explicit cases, plus a set-size assertion or an exhaustive `match` so new members cannot be silently omitted.
2. A literal-only `(input, expected)` table with a loop containing a single assertion.
3. A `macro_rules!` that stamps out the same test per type, as a last resort when the axis is a type.

Do not use branches inside case loops, expectations derived from rows, nested loops generating a product, or helpers that decide per case. `cli/tests/cli/combination.rs` is the shape to avoid. Preserve every existing case when consolidating tests.
