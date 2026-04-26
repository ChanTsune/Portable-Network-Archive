# `lib/src/cipher/aead/` — AEAD Prototype Skeleton

> **Status: Prototype skeleton (2026-04-26, post minimal-redesign).**
> These files are NOT yet wired into the build (`lib/src/cipher.rs` does not declare `mod aead;` yet).
> They exist as scaffolding for the AEAD migration described in `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md` (now superseded by the 2026-04-26 minimal redesign — see header note in that doc).

## Purpose

This directory will contain the implementation of authenticated encryption (AEAD) for libpna, addressing finding **CRY-001** from the 2026-04-23 security audit.

## Contents

| File | Status | Role |
|---|---|---|
| `mod.rs` (i.e., `aead.rs` per Rust convention) | TODO | AeadContext, AAD construction, nonce generation, common helpers |
| `aes_gcm.rs` | TODO | Wraps `aes-gcm` crate (RustCrypto, NCC Group audited 2020) |
| `camellia_gcm.rs` | TODO | Camellia-256-GCM via `aes_gcm::AesGcm<Camellia256, U12>` (3-line composition; the crate name is a misnomer) |
| `read.rs` | TODO | `AeadReader<R>`: per-chunk tag verification, RUP defense |
| `write.rs` | TODO | `AeadWriter<W>`: per-chunk encrypt + tag append |

## Activation

When ready to begin actual implementation:

1. Add to `lib/Cargo.toml`:
   ```toml
   aes-gcm = "0.10"
   camellia = "0.2"
   zeroize = "1.7"
   subtle = "2.5"
   ```
2. Add to `lib/src/cipher.rs`:
   ```rust
   mod aead;
   ```
3. Wire up `CipherWriter::GcmAes` / `GcmCamellia` and `DecryptReader` counterparts.
4. Plumb the entry's PHSF chunk output (32-byte Argon2id/PBKDF2 result) directly into `AeadContext::key`. **No further key derivation step** is applied — see spec §4.1.5 PHSF AEAD note.

## References

- Spec: `Portable-Network-Archive-Specification` repo, branch `spec/aead-gcm-introduction`, commit `640936b` and later:
  - `cipher_modes/index.md` §7.4 GCM, §7.5 nonce/tag placement (12B inline nonce + 16B tag)
  - `chunk_specifications/index.md` §4.1.4 FHED Cipher mode = 2 (GCM); §4.1.5 PHSF AEAD note (output used directly as GCM key)
  - `key_derivation_algorithms/index.md` §8.1.5 PBKDF2 / §8.2.5 Argon2 AEAD notes (32-byte output is the GCM key)
  - `recommendations_for_decoders/index.md` §12.3 AEAD-specific decoder behavior
  - Design doc: `docs/superpowers/specs/2026-04-26-aead-minimal-redesign.md`
- Plan document (superseded): `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md`
- NIST SP 800-38D (GCM normative spec): https://csrc.nist.gov/pubs/sp/800/38/d/final
- RFC 6367 (Camellia in TLS, including Camellia-GCM): https://datatracker.ietf.org/doc/html/rfc6367
