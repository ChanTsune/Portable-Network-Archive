# `lib/src/cipher/aead/` — AEAD Prototype Skeleton

> **Status: Prototype skeleton (2026-04-24).**
> These files are NOT yet wired into the build (`lib/src/cipher.rs` does not declare `mod aead;` yet).
> They exist as scaffolding for the AEAD migration described in `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md`.

## Purpose

This directory will contain the implementation of authenticated encryption (AEAD) for libpna, addressing finding **CRY-001** from the 2026-04-23 security audit.

## Contents

| File | Status | Role |
|---|---|---|
| `mod.rs` (i.e., `aead.rs` per Rust convention) | TODO | AeadContext, AAD construction, nonce derivation, common helpers |
| `aes_gcm.rs` | TODO | Wraps `aes-gcm` crate (RustCrypto, NCC Group audited 2020) |
| `camellia_gcm.rs` | TODO | DIY Camellia-256-GCM via `camellia` + `ctr` + `ghash` composition |
| `read.rs` | TODO | `AeadReader<R>`: per-chunk tag verification, RUP defense |
| `write.rs` | TODO | `AeadWriter<W>`: per-chunk encrypt + tag append |

## Activation

When ready to begin actual implementation:

1. Add to `lib/Cargo.toml`:
   ```toml
   aes-gcm = "0.10"
   ghash = "0.6"
   hkdf = "0.13"
   zeroize = "1.7"
   subtle = "2.5"
   ```
2. Add to `lib/src/cipher.rs`:
   ```rust
   mod aead;
   ```
3. Wire up `CipherWriter::GcmAes` / `GcmCamellia` and `DecryptReader` counterparts.
4. Implement per-entry HKDF subkey derivation in `lib/src/entry/{read,write}.rs`.
5. Define `AENC` and `FENC` chunk types in `lib/src/chunk/types.rs`.

## References

- Spec drafts in `Portable-Network-Archive-Specification` repo, branch `spec/aead-gcm-introduction`:
  - `cipher_modes/index.md` §7.4 GCM, §7.5 nonce/tag placement
  - `chunk_specifications/index.md` AENC (per-archive), FENC (per-entry) chunks
  - `key_derivation_algorithms/index.md` §8.3 HKDF
  - `recommendations_for_decoders/index.md` §12.3 AEAD-specific decoder behavior
- Plan document: `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md`
- NIST SP 800-38D (GCM normative spec): https://csrc.nist.gov/pubs/sp/800/38/d/final
- RFC 6367 (Camellia in TLS, including Camellia-GCM): https://datatracker.ietf.org/doc/html/rfc6367
- RFC 5869 (HKDF): https://datatracker.ietf.org/doc/html/rfc5869
