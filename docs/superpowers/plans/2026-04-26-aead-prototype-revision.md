# libpna AEAD Prototype Revision Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Revise the libpna AEAD prototype skeleton (`lib/src/cipher/aead.rs` + 5 sibling files) to match the finalized minimal AEAD design (per-entry, 11-byte AAD, random nonce inline, no HKDF, zero new chunk types).

**Architecture:** Targeted edits within the existing `lib/src/cipher/aead/` module tree. The prototype is **not yet wired into the build** (no `mod aead;` in `lib/src/cipher.rs`), so revisions touch only the prototype files and their docs. Testing scope is limited to `cargo build` (skeleton compiles even with `unimplemented!()` bodies) and the existing 5 unit tests in `aead.rs` (rewritten to match the new struct layout).

**Tech Stack:** Rust 2024, libpna workspace. Will use `rand_core::CryptoRng` for nonce generation (RustCrypto common trait; concrete impl deferred to actual integration). No new crate dependencies introduced; `hkdf = "0.13"` is removed from the activation steps.

**Branch:** Continue on `lib/aead-prototype` (currently at commit `c063688f`). All changes land on this branch.

**Source spec:** `Portable-Network-Archive-Specification` repo, branch `spec/aead-gcm-introduction`, commit `640936b` (latest, applies the minimal redesign). Design rationale: `docs/superpowers/specs/2026-04-26-aead-minimal-redesign.md` in that repo.

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `lib/src/cipher/aead.rs` | Module root: AeadContext, AAD construction, nonce, error types, unit tests | Major rewrite. Remove ArchiveContext, EntryContext, AEAD_AAD_MAGIC, AEAD_FRAMEWORK_VERSION. Shrink AAD from 86B to 11B. Replace deterministic build_nonce with CSPRNG generate_nonce. Rewrite 5 unit tests for new layout. |
| `lib/src/cipher/aead/README.md` | Activation guide for the prototype | Update spec link table (remove §8.3 HKDF, AENC, FENC entries). Remove `hkdf = "0.13"` from Cargo.toml steps. Remove "Define AENC and FENC chunk types" step. |
| `lib/src/cipher/aead/aes_gcm.rs` | AES-256-GCM wrapper (skeleton stubs) | Update doc comments: chunk data layout `[12B nonce][ct][16B tag]`. API signature unchanged (functions still accept ctx + chunk_index + is_final + data). |
| `lib/src/cipher/aead/camellia_gcm.rs` | Camellia-256-GCM (skeleton stubs) | Same as aes_gcm.rs: doc-comment-only updates for new chunk layout. The "Implementation pitfalls" section stays valid (constant-time tag, RUP, etc.). |
| `lib/src/cipher/aead/read.rs` | AEAD chunked decryption reader (skeleton) | Update TODO list in `Read::read` impl: nonce comes from chunk's first 12 bytes (not from ctx-derived). |
| `lib/src/cipher/aead/write.rs` | AEAD chunked encryption writer (skeleton) | Update TODO list in `Write::write` impl: nonce generated per chunk via CSPRNG (not derived from ctx). Emit `(nonce ‖ ct ‖ tag)`. |
| `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md` | Pre-revision planning document (assumed: contains 86B AAD + HKDF design) | Add a header note marking the doc as superseded by the 2026-04-26 minimal redesign. Do NOT delete; preserve as historical record. |

---

## Task 1: Rewrite `lib/src/cipher/aead.rs`

This is the central revision. The current file is 277 lines; the revised file will be ~190 lines (removed: ArchiveContext, EntryContext, magic constant, framework_version, metadata_hash, deterministic nonce derivation; added: CSPRNG nonce generation, simpler 11-byte AAD).

**Files:**
- Modify: `lib/src/cipher/aead.rs` (whole-file rewrite)

- [ ] **Step 1: Replace the file contents in one Write operation**

Use Write on `lib/src/cipher/aead.rs` with the following content. (Whole-file rewrite is cleaner than a series of Edits because nearly every section changes.)

```rust
//! Authenticated Encryption with Associated Data (AEAD) for PNA.
//!
//! **Status: prototype skeleton (2026-04-26, post minimal-redesign).**
//! This module is not yet wired into the build. See
//! `lib/src/cipher/aead/README.md` for activation instructions.
//!
//! This module implements the AEAD construction defined in the PNA specification
//! draft `spec/aead-gcm-introduction`:
//!
//! - [§7.4 GCM] — algorithm, parameters, AAD, nonce, key/nonce uniqueness
//! - [§7.5] — FDAT/SDAT layout under AEAD (`12B random nonce || ciphertext || 16B tag`)
//! - [§4.1.5 PHSF] — password-hash output is used directly as the per-entry GCM key
//!
//! [§7.4 GCM]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/cipher_modes/index.md
//! [§7.5]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/cipher_modes/index.md
//! [§4.1.5 PHSF]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/chunk_specifications/index.md
//!
//! # Threat model
//!
//! Addresses finding **CRY-001** (unauthenticated encryption) from the 2026-04-23
//! security audit.
//!
//! # Implementation strategy
//!
//! Per design decision 2026-04-26 (minimal redesign):
//! - AES-256-GCM via `aes-gcm` crate (`Aes256Gcm`).
//! - Camellia-256-GCM via `aes_gcm::AesGcm<Camellia256, U12>` (3-line composition,
//!   leveraging the generic GCM machinery; the crate name is a misnomer).
//! - Per-entry GCM key = PHSF output (Argon2id or PBKDF2, 32 bytes), used directly.
//! - Per-chunk 12-byte nonce generated via CSPRNG; stored inline as the first
//!   12 bytes of each FDAT/SDAT chunk's data field.
//! - AAD = 11 bytes: encryption_byte || mode_byte || entry_index_be ||
//!   chunk_index_be || final_flag.
//! - **No** AENC/FENC chunk types. **No** HKDF derivation. **No** archive-level
//!   AEAD context state.

#![allow(dead_code)] // skeleton, not yet wired in

mod aes_gcm;
mod camellia_gcm;
mod read;
mod write;

/// Total AAD length per chunk.
///
/// See spec §7.4.2. Layout (11 bytes):
/// `encryption_byte (1) || mode_byte (1) || entry_index_be (4) || chunk_index_be (4) || final_flag (1)`.
pub(crate) const AEAD_AAD_LEN: usize = 11;

/// Per-chunk nonce length in bytes.
///
/// Required by spec §7.4.1 (96-bit nonces only) and NIST SP 800-38D §5.2.1.1.
pub(crate) const AEAD_NONCE_LEN: usize = 12;

/// GCM authentication tag length in bytes.
///
/// Required by spec §7.4.1 (full 128-bit tags only; truncated tags forbidden).
pub(crate) const AEAD_TAG_LEN: usize = 16;

/// Per-entry AEAD execution context.
///
/// Holds derived state needed for encrypting/decrypting a single entry's chunks.
/// Constructed once per entry from the entry's PHSF chunk; reused for every
/// FDAT/SDAT chunk of that entry.
#[derive(Debug, Clone)]
pub(crate) struct AeadContext {
    /// Per-entry GCM key (32 bytes).
    ///
    /// Equal to the entry's PHSF password-hash output (Argon2id or PBKDF2 output).
    /// No further key derivation is applied. See spec §4.1.5 (PHSF AEAD note).
    pub key: [u8; 32],
    /// FHED `Encryption method` byte (1 = AES, 2 = Camellia).
    pub encryption_byte: u8,
    /// FHED `Cipher mode` byte (= 2 for GCM in PNA Minor 1).
    pub mode_byte: u8,
    /// 0-indexed position of this entry within the archive.
    ///
    /// Authenticated via AAD; defends against intra-archive entry reorder
    /// and cross-entry chunk swap attacks.
    pub entry_index: u32,
}

impl AeadContext {
    /// Construct the 11-byte AAD for a given chunk.
    ///
    /// Per spec §7.4.2:
    /// ```text
    /// AAD[0]      = encryption_byte
    /// AAD[1]      = mode_byte
    /// AAD[2..6]   = entry_index (u32 BE)
    /// AAD[6..10]  = chunk_index (u32 BE)
    /// AAD[10]     = if is_final { 0x01 } else { 0x00 }
    /// ```
    pub(crate) fn build_aad(&self, chunk_index: u32, is_final: bool) -> [u8; AEAD_AAD_LEN] {
        let mut aad = [0u8; AEAD_AAD_LEN];
        aad[0] = self.encryption_byte;
        aad[1] = self.mode_byte;
        aad[2..6].copy_from_slice(&self.entry_index.to_be_bytes());
        aad[6..10].copy_from_slice(&chunk_index.to_be_bytes());
        aad[10] = if is_final { 0x01 } else { 0x00 };
        aad
    }
}

/// Generate a fresh 12-byte nonce from a cryptographically secure RNG.
///
/// Per spec §7.4.3 / §7.4.5: GCM nonces in PNA are random (NIST SP 800-38D
/// §8.2.2 RBG-based construction), one fresh nonce per chunk.
///
/// Encoders MUST surface CSPRNG failure and abort archive creation rather than
/// silently producing weak random values (spec §7.4.5).
pub(crate) fn generate_nonce<R: rand_core::RngCore + rand_core::CryptoRng>(
    rng: &mut R,
) -> [u8; AEAD_NONCE_LEN] {
    let mut nonce = [0u8; AEAD_NONCE_LEN];
    rng.fill_bytes(&mut nonce);
    nonce
}

/// AEAD authentication failure error.
///
/// Distinct from I/O errors and format errors per spec §12.3.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuthFailure;

impl core::fmt::Display for AuthFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AEAD authentication tag verification failed")
    }
}

impl std::error::Error for AuthFailure {}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_context() -> AeadContext {
        AeadContext {
            key: [0u8; 32],
            encryption_byte: 1, // AES
            mode_byte: 2,       // GCM
            entry_index: 0x0102_0304,
        }
    }

    #[test]
    fn build_aad_length_11_bytes_per_spec_7_4_2() {
        let ctx = dummy_context();
        let aad = ctx.build_aad(0, false);
        assert_eq!(aad.len(), AEAD_AAD_LEN);
        assert_eq!(aad.len(), 11);
    }

    #[test]
    fn build_aad_field_layout_offsets() {
        let ctx = dummy_context();
        let aad = ctx.build_aad(0x0506_0708, false);
        // Offset table per spec §7.4.2 (11 bytes total):
        //   0:      encryption_byte (= 1, AES)
        //   1:      mode_byte (= 2, GCM)
        //   2..6:   entry_index (= 0x0102_0304)
        //   6..10:  chunk_index (= 0x0506_0708)
        //   10:     final flag (0x00 here)
        assert_eq!(aad[0], 1);
        assert_eq!(aad[1], 2);
        assert_eq!(&aad[2..6], &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(&aad[6..10], &[0x05, 0x06, 0x07, 0x08]);
        assert_eq!(aad[10], 0x00);
    }

    #[test]
    fn build_aad_final_chunk_flag_distinguishes_last_chunk() {
        let ctx = dummy_context();
        let aad_nonfinal = ctx.build_aad(5, false);
        let aad_final = ctx.build_aad(5, true);
        assert_eq!(aad_nonfinal[10], 0x00);
        assert_eq!(aad_final[10], 0x01);
        // All other bytes identical
        assert_eq!(&aad_nonfinal[..10], &aad_final[..10]);
    }

    #[test]
    fn build_aad_entry_index_distinguishes_entries() {
        // Defense against cross-entry chunk swap: same chunk_index in two
        // different entries must produce DIFFERENT AAD.
        let ctx_entry_0 = AeadContext {
            entry_index: 0,
            ..dummy_context()
        };
        let ctx_entry_1 = AeadContext {
            entry_index: 1,
            ..dummy_context()
        };
        let aad_0 = ctx_entry_0.build_aad(5, false);
        let aad_1 = ctx_entry_1.build_aad(5, false);
        assert_ne!(aad_0, aad_1);
        // Diff is in entry_index field (offset 2..6)
        assert_eq!(&aad_0[..2], &aad_1[..2]);
        assert_ne!(&aad_0[2..6], &aad_1[2..6]);
        assert_eq!(&aad_0[6..], &aad_1[6..]);
    }

    #[test]
    fn generate_nonce_returns_12_bytes() {
        // Use a deterministic test RNG to make the assertion reproducible.
        struct TestRng(u8);
        impl rand_core::RngCore for TestRng {
            fn next_u32(&mut self) -> u32 {
                unimplemented!()
            }
            fn next_u64(&mut self) -> u64 {
                unimplemented!()
            }
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                for b in dest {
                    *b = self.0;
                    self.0 = self.0.wrapping_add(1);
                }
            }
            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
                self.fill_bytes(dest);
                Ok(())
            }
        }
        impl rand_core::CryptoRng for TestRng {}

        let mut rng = TestRng(0xA0);
        let nonce = generate_nonce(&mut rng);
        assert_eq!(nonce.len(), AEAD_NONCE_LEN);
        assert_eq!(
            nonce,
            [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xAB]
        );
    }
}
```

- [ ] **Step 2: Verify the rewritten file compiles**

Run from libpna repo root:

```bash
cd /Users/tsunekawa/Documents/GitHub/Portable-Network-Archive/.claude/worktrees/magical-frolicking-cocke
cargo check -p libpna 2>&1 | tail -20
```

Expected: clean compile (no errors). Warnings about `dead_code` are expected and silenced by the `#![allow(dead_code)]` attribute.

If `cargo check` fails with `error[E0432]: unresolved import \`rand_core\``: `rand_core` is a transitive dep of existing crates (e.g., via `aes`), so it should resolve. If not, fall back to a smaller change: replace the `generate_nonce` function with a stub that takes `&mut [u8; 12]` and accept the caller will fill it. (This deferral is acceptable for a skeleton; the real CSPRNG plumbing happens at activation time.)

To check rand_core availability:

```bash
cargo tree -p libpna -i rand_core 2>&1 | head -10
```

Expected: at least one inverse dependency listed (e.g., `rand_core via X`). If empty, switch to the stub-style API per the fallback note above.

- [ ] **Step 3: Run the unit tests**

The skeleton's unit tests are NOT in the build (parent `lib/src/cipher.rs` does not declare `mod aead`), so `cargo test` will not pick them up. Verify the test code is at least syntactically valid by running the file through rustfmt:

```bash
cargo fmt --check -- lib/src/cipher/aead.rs
```

Expected: no diff (the rewritten content already matches rustfmt 2024 defaults).

If rustfmt reports diffs, run `cargo fmt -- lib/src/cipher/aead.rs` to fix and re-check.

---

## Task 2: Update `lib/src/cipher/aead/README.md`

**Files:**
- Modify: `lib/src/cipher/aead/README.md` (entire file rewrite due to scope of changes)

- [ ] **Step 1: Replace the README contents in one Write operation**

Use Write on `lib/src/cipher/aead/README.md` with:

```markdown
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
4. Plumb the entry's PHSF chunk output (32-byte Argon2id/PBKDF2 result) directly into `AeadContext::key`. **No HKDF derivation** is applied — see spec §4.1.5 PHSF AEAD note.

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
```

- [ ] **Step 2: Verify the README renders without spec-link drift**

Run from libpna repo root:

```bash
cd /Users/tsunekawa/Documents/GitHub/Portable-Network-Archive/.claude/worktrees/magical-frolicking-cocke
grep -n "AENC\|FENC\|HKDF\|RFC 5869\|hkdf = " lib/src/cipher/aead/README.md
```

Expected: no output. If matches appear, the rewrite did not fully replace the old content.

---

## Task 3: Update `lib/src/cipher/aead/aes_gcm.rs` doc comments

**Files:**
- Modify: `lib/src/cipher/aead/aes_gcm.rs:42-48` (`decrypt_chunk` doc comment — chunk layout description)

- [ ] **Step 1: Update the chunk layout comment in `decrypt_chunk`**

Use Edit on `lib/src/cipher/aead/aes_gcm.rs`:

```
old_string:
/// Decrypt and verify a single chunk under AES-256-GCM.
///
/// `chunk_data` is `(ciphertext || 16-byte tag)` per spec §7.5.1.
/// Returns plaintext only if tag verification succeeds.
///
/// **Not yet implemented.**
pub(crate) fn decrypt_chunk(
    _ctx: &AeadContext,
    _chunk_index: u32,
    _is_final: bool,
    _chunk_data: &[u8],
) -> Result<Vec<u8>, AuthFailure> {
    unimplemented!("AES-256-GCM decryption not yet implemented; see README.md for activation steps")
}

new_string:
/// Decrypt and verify a single chunk under AES-256-GCM.
///
/// `chunk_data` is `(12-byte nonce || ciphertext || 16-byte tag)` per spec §7.5.1.
/// The first 12 bytes are read as the nonce; the last 16 bytes are the
/// authentication tag; the middle is the ciphertext.
/// Returns plaintext only if tag verification succeeds.
///
/// **Not yet implemented.**
pub(crate) fn decrypt_chunk(
    _ctx: &AeadContext,
    _chunk_index: u32,
    _is_final: bool,
    _chunk_data: &[u8],
) -> Result<Vec<u8>, AuthFailure> {
    unimplemented!("AES-256-GCM decryption not yet implemented; see README.md for activation steps")
}
```

- [ ] **Step 2: Update the `encrypt_chunk` return doc**

Use Edit on `lib/src/cipher/aead/aes_gcm.rs`:

```
old_string:
/// Encrypt a single chunk under AES-256-GCM.
///
/// Returns ciphertext (same length as plaintext) appended with the 16-byte tag.
///
/// **Not yet implemented.** Will use `aes_gcm::Aes256Gcm::new(...).encrypt(nonce, payload)`
/// where `payload.aad = ctx.build_aad(chunk_index, is_final)`.
pub(crate) fn encrypt_chunk(

new_string:
/// Encrypt a single chunk under AES-256-GCM.
///
/// Returns the chunk's full data field: `(12-byte nonce || ciphertext || 16-byte tag)`.
/// The nonce is generated by a CSPRNG inside this function (see spec §7.4.3 / §7.4.5);
/// the caller does not provide it.
///
/// **Not yet implemented.** Will use `aes_gcm::Aes256Gcm::new(...).encrypt(nonce, payload)`
/// where `payload.aad = ctx.build_aad(chunk_index, is_final)` and `nonce` is `super::generate_nonce(rng)`.
pub(crate) fn encrypt_chunk(
```

- [ ] **Step 3: Update the implementation TODO list at the top of the file**

Use Edit on `lib/src/cipher/aead/aes_gcm.rs`:

```
old_string:
//! # Implementation TODO
//!
//! 1. Add `aes-gcm = "0.10"` to `lib/Cargo.toml`.
//! 2. Implement `encrypt_chunk` / `decrypt_chunk` using `aes_gcm::Aes256Gcm`.
//! 3. Plumb `AeadContext::build_nonce` and `AeadContext::build_aad`.
//! 4. Add NIST CAVP test vectors in `lib/tests/aead/aes_gcm_nist_cavp.rs`.
//! 5. Add OpenSSL CLI cross-validation in `lib/tests/aead/aes_gcm_openssl_compat.rs`.

new_string:
//! # Implementation TODO
//!
//! 1. Add `aes-gcm = "0.10"` to `lib/Cargo.toml`.
//! 2. Implement `encrypt_chunk` / `decrypt_chunk` using `aes_gcm::Aes256Gcm`.
//! 3. Plumb `super::generate_nonce` (CSPRNG) into encrypt; read first 12 bytes of
//!    chunk_data as nonce in decrypt. Plumb `AeadContext::build_aad` for both.
//! 4. Add NIST CAVP test vectors in `lib/tests/aead/aes_gcm_nist_cavp.rs`.
//! 5. Add OpenSSL CLI cross-validation in `lib/tests/aead/aes_gcm_openssl_compat.rs`.
```

- [ ] **Step 4: Verify**

Run:

```bash
cd /Users/tsunekawa/Documents/GitHub/Portable-Network-Archive/.claude/worktrees/magical-frolicking-cocke
cargo fmt --check -- lib/src/cipher/aead/aes_gcm.rs
grep -n "build_nonce\|16-byte tag" lib/src/cipher/aead/aes_gcm.rs
```

Expected:
- rustfmt: no diff
- grep for `build_nonce`: no output (we removed it)
- grep for `16-byte tag`: at least one match (still mentioned correctly in layout descriptions)

---

## Task 4: Update `lib/src/cipher/aead/camellia_gcm.rs` doc comments

The "Implementation pitfalls (must avoid)" section is still entirely valid (constant-time tag, RUP, GHASH padding, counter endianness) and should remain unchanged. Only the chunk-layout doc comments need updating.

**Files:**
- Modify: `lib/src/cipher/aead/camellia_gcm.rs:73-96` (encrypt_chunk + decrypt_chunk doc comments)

- [ ] **Step 1: Update the `encrypt_chunk` doc**

Use Edit on `lib/src/cipher/aead/camellia_gcm.rs`:

```
old_string:
/// Encrypt a single chunk under Camellia-256-GCM.
///
/// Returns ciphertext (same length as plaintext) appended with the 16-byte tag.
///
/// **Not yet implemented.**
pub(crate) fn encrypt_chunk(

new_string:
/// Encrypt a single chunk under Camellia-256-GCM.
///
/// Returns the chunk's full data field: `(12-byte nonce || ciphertext || 16-byte tag)`.
/// The nonce is generated by a CSPRNG inside this function (see spec §7.4.3 / §7.4.5);
/// the caller does not provide it.
///
/// **Not yet implemented.**
pub(crate) fn encrypt_chunk(
```

- [ ] **Step 2: Update the `decrypt_chunk` doc**

Use Edit on `lib/src/cipher/aead/camellia_gcm.rs`:

```
old_string:
/// Decrypt and verify a single chunk under Camellia-256-GCM.
///
/// **Not yet implemented.**
pub(crate) fn decrypt_chunk(

new_string:
/// Decrypt and verify a single chunk under Camellia-256-GCM.
///
/// `chunk_data` is `(12-byte nonce || ciphertext || 16-byte tag)` per spec §7.5.1.
///
/// **Not yet implemented.**
pub(crate) fn decrypt_chunk(
```

- [ ] **Step 3: Verify**

Run:

```bash
cd /Users/tsunekawa/Documents/GitHub/Portable-Network-Archive/.claude/worktrees/magical-frolicking-cocke
cargo fmt --check -- lib/src/cipher/aead/camellia_gcm.rs
```

Expected: no diff.

---

## Task 5: Update `lib/src/cipher/aead/read.rs` TODO comment

**Files:**
- Modify: `lib/src/cipher/aead/read.rs:47-58` (the `Read::read` TODO list)

- [ ] **Step 1: Update the TODO list inside `Read::read`**

Use Edit on `lib/src/cipher/aead/read.rs`:

```
old_string:
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        // TODO: per spec §12.3.2/12.3.3:
        //   1. Read next chunk header (length, type, ...) from inner.
        //   2. Read full chunk data (ciphertext || 16-byte tag).
        //   3. Compute AAD: ctx.build_aad(chunk_index, is_final_assumption).
        //   4. Verify tag using `aes_gcm::Aes256Gcm` or `camellia_gcm::CamelliaGcm256`.
        //   5. If verification fails, return InvalidData; do NOT release plaintext.
        //   6. If success, append decrypted plaintext to pending buffer; serve from buffer.
        //   7. On reaching FEND/SEND without seeing is_final_chunk = 0x01 set, return InvalidData (truncation).
        unimplemented!()
    }

new_string:
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        // TODO: per spec §12.3.2/12.3.3:
        //   1. Read next chunk header (length, type, ...) from inner.
        //   2. Read full chunk data (12-byte nonce || ciphertext || 16-byte tag).
        //      Reject any chunk whose data length is < 28 bytes (12 + 0 + 16).
        //   3. Extract first 12 bytes as nonce; last 16 bytes as tag; middle as ciphertext.
        //   4. Compute AAD: ctx.build_aad(chunk_index, is_final_assumption).
        //   5. Verify tag using `aes_gcm::Aes256Gcm` or `camellia_gcm::CamelliaGcm256`.
        //   6. If verification fails, return InvalidData; do NOT release plaintext (RUP).
        //   7. If success, append decrypted plaintext to pending buffer; serve from buffer.
        //   8. On reaching FEND/SEND without seeing Final-chunk flag = 0x01 authenticated,
        //      return InvalidData (truncation).
        unimplemented!()
    }
```

- [ ] **Step 2: Verify**

Run:

```bash
cd /Users/tsunekawa/Documents/GitHub/Portable-Network-Archive/.claude/worktrees/magical-frolicking-cocke
cargo fmt --check -- lib/src/cipher/aead/read.rs
grep -n "is_final_chunk" lib/src/cipher/aead/read.rs
```

Expected:
- rustfmt: no diff
- grep for `is_final_chunk`: no output (renamed to `Final-chunk flag`)

---

## Task 6: Update `lib/src/cipher/aead/write.rs` TODO comment

**Files:**
- Modify: `lib/src/cipher/aead/write.rs:7-8` (module-level doc with `is_final_chunk`)
- Modify: `lib/src/cipher/aead/write.rs:60-79` (`Write::write` TODO list)

- [ ] **Step 1: Update the module-level doc comment**

Use Edit on `lib/src/cipher/aead/write.rs`:

```
old_string:
//! Per spec §7.5.3, encoders MUST chunk plaintext into uniform-size segments
//! (recommended 64 KiB) and emit each as one FDAT/SDAT chunk whose data is
//! `(ciphertext || 16-byte tag)`. The final chunk preceding FEND/SEND MUST
//! be marked with the AAD `is_final_chunk` flag = `0x01`.

new_string:
//! Per spec §7.5.3, encoders MUST chunk plaintext into uniform-size segments
//! (recommended 64 KiB) and emit each as one FDAT/SDAT chunk whose data is
//! `(12-byte CSPRNG nonce || ciphertext || 16-byte tag)`. The final chunk
//! preceding FEND/SEND MUST be marked with the AAD Final-chunk flag = `0x01`.
```

- [ ] **Step 2: Update the TODO list inside `Write::write`**

Use Edit on `lib/src/cipher/aead/write.rs`:

```
old_string:
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        // TODO: per spec §7.5.3:
        //   1. Append _buf to _chunk_buffer.
        //   2. While _chunk_buffer.len() >= _chunk_size:
        //      a. Take first _chunk_size bytes as plaintext.
        //      b. Compute AAD: _ctx.build_aad(_chunk_index, false).
        //      c. Compute nonce: _ctx.build_nonce(_chunk_index).
        //      d. Encrypt under chosen algorithm; get (ciphertext, tag).
        //      e. Emit one chunk to _inner: data = (ciphertext || tag).
        //      f. _chunk_index += 1.
        //   3. Return _buf.len() as bytes written.
        unimplemented!()
    }

new_string:
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        // TODO: per spec §7.5.3:
        //   1. Append _buf to _chunk_buffer.
        //   2. While _chunk_buffer.len() >= _chunk_size:
        //      a. Take first _chunk_size bytes as plaintext.
        //      b. Compute AAD: _ctx.build_aad(_chunk_index, false).
        //      c. Generate fresh nonce: super::generate_nonce(&mut rng) (CSPRNG, 12B).
        //      d. Encrypt under chosen algorithm; get (ciphertext, tag).
        //      e. Emit one chunk to _inner: data = (nonce || ciphertext || tag).
        //      f. _chunk_index += 1.
        //   3. Return _buf.len() as bytes written.
        unimplemented!()
    }
```

- [ ] **Step 3: Verify**

Run:

```bash
cd /Users/tsunekawa/Documents/GitHub/Portable-Network-Archive/.claude/worktrees/magical-frolicking-cocke
cargo fmt --check -- lib/src/cipher/aead/write.rs
grep -n "build_nonce\|is_final_chunk" lib/src/cipher/aead/write.rs
```

Expected:
- rustfmt: no diff
- grep: no output (`build_nonce` replaced with `generate_nonce`; `is_final_chunk` renamed)

---

## Task 7: Mark `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md` as superseded

The 2026-04-24 plan doc was authored before the minimal redesign and contains the old AENC/FENC/HKDF design. Add a header note so future readers know which sections are stale, but do NOT delete the doc — it preserves the design history (including why we considered the prior approach).

**Files:**
- Modify: `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md` (add a 4-line note at the top, immediately after the H1 title)

- [ ] **Step 1: Insert a superseded note after the H1**

The current H1 of `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md` is:

```
# libpna AEAD Migration Plan: AES-256-GCM + Camellia-256-GCM (Phase 1)
```

followed by a blank line and then a `> **Status**: ...` block. We insert the superseded note between the H1 and the existing `Status` block.

Use Edit on `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md`:

```
old_string:
# libpna AEAD Migration Plan: AES-256-GCM + Camellia-256-GCM (Phase 1)

> **Status**: Design discussion outcome, pre-implementation. Captures decisions from the 2026-04-23/24 design session.

new_string:
# libpna AEAD Migration Plan: AES-256-GCM + Camellia-256-GCM (Phase 1)

> **⚠️ Superseded by the 2026-04-26 minimal redesign.**
>
> This document was authored before the AEAD minimal redesign of 2026-04-26. Sections describing AENC/FENC chunks, HKDF per-entry key derivation, and the 86-byte AAD are no longer current. The actual implementation follows the design in the spec repo's `docs/superpowers/specs/2026-04-26-aead-minimal-redesign.md` and the libpna prototype rewrite in commit `<TBD: filled in by Task 8 commit hash>`. This doc is preserved for historical context.

> **Status**: Design discussion outcome, pre-implementation. Captures decisions from the 2026-04-23/24 design session.
```

(The `<TBD: filled in by Task 8 commit hash>` placeholder is replaced AFTER the Task 8 commit lands. See Task 8 Step 6.)

- [ ] **Step 2: Verify the note is present**

Run:

```bash
grep -n "Superseded by the 2026-04-26 minimal redesign" /Users/tsunekawa/Documents/GitHub/Portable-Network-Archive/.claude/worktrees/magical-frolicking-cocke/docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md
```

Expected: 1 match, near the top (line 3 or 4).

---

## Task 8: Final verification + atomic commit

- [ ] **Step 1: Pre-commit hygiene**

Run from libpna repo root:

```bash
cd /Users/tsunekawa/Documents/GitHub/Portable-Network-Archive/.claude/worktrees/magical-frolicking-cocke
pwd
git remote -v
git branch --show-current
git status
```

Expected:
- `pwd`: `.../Portable-Network-Archive/.claude/worktrees/magical-frolicking-cocke`
- `git remote -v`: includes `origin` pointing to `Portable-Network-Archive`
- Branch: `lib/aead-prototype`
- Modified: 6 files (aead.rs, aead/README.md, aead/aes_gcm.rs, aead/camellia_gcm.rs, aead/read.rs, aead/write.rs, security-audits/...plan-2026-04-24.md). Untracked: docs/superpowers/plans/2026-04-26-aead-prototype-revision.md (this plan).

- [ ] **Step 2: Whole-prototype grep for stale terms**

Run:

```bash
grep -rn "AENC\|FENC\|HKDF\|archive_identifier\|entry_random\|PNA-AEAD-v1\|metadata_hash\|AEAD_AAD_MAGIC\|AEAD_FRAMEWORK_VERSION\|ArchiveContext\|EntryContext\|build_nonce\|is_final_chunk" lib/src/cipher/aead.rs lib/src/cipher/aead/
```

Expected: no output. Any remaining match means a Task above missed something — fix and re-grep.

- [ ] **Step 3: Cargo build sanity check**

Run:

```bash
cargo check --workspace 2>&1 | tail -10
```

Expected: clean (the prototype is `#![allow(dead_code)]` and not yet declared as a module, so it compiles in isolation; `cargo check --workspace` does not pull it into the build either way, but a clean output confirms no other regressions).

If the prototype is somehow now part of the build (someone added `mod aead;` since this plan was written), and `rand_core` is unavailable, fall back to the stub-style API mentioned in Task 1 Step 2.

- [ ] **Step 4: Stage all modified files and the new plan**

Run:

```bash
git add lib/src/cipher/aead.rs \
        lib/src/cipher/aead/README.md \
        lib/src/cipher/aead/aes_gcm.rs \
        lib/src/cipher/aead/camellia_gcm.rs \
        lib/src/cipher/aead/read.rs \
        lib/src/cipher/aead/write.rs \
        docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md \
        docs/superpowers/plans/2026-04-26-aead-prototype-revision.md

git diff --cached --stat
```

Expected: 8 files changed.

- [ ] **Step 5: Commit**

Run:

```bash
git commit -m ":memo: Revise AEAD prototype skeleton per minimal redesign" -m "Aligns the lib/src/cipher/aead/ skeleton with the 2026-04-26 PNA spec
minimal redesign (per-entry AEAD, 11-byte AAD, random inline nonce,
no HKDF, no AENC/FENC chunks).

- aead.rs: remove ArchiveContext, EntryContext, AEAD_AAD_MAGIC,
  AEAD_FRAMEWORK_VERSION, metadata_hash. Shrink AAD from 86B to 11B.
  Replace deterministic build_nonce with CSPRNG generate_nonce.
  Rewrite 5 unit tests for the new struct layout.
- aead/README.md: drop hkdf dep + AENC/FENC chunk type definition step.
  Update spec link table to point to revised spec sections.
- aead/{aes_gcm,camellia_gcm}.rs: update doc comments to describe new
  chunk layout (12B nonce || ct || 16B tag) and CSPRNG nonce sourcing.
- aead/{read,write}.rs: update TODO comments for the new layout and
  the renamed Final-chunk flag (was is_final_chunk).
- security-audits/...plan-2026-04-24.md: add superseded note at top;
  preserve doc as historical record.

Spec source: Portable-Network-Archive-Specification@640936b
(spec/aead-gcm-introduction branch)."
```

- [ ] **Step 6: Backfill the commit hash into the superseded note**

The note in `docs/security-audits/...plan-2026-04-24.md` contains a placeholder `<TBD: filled in by Task 8 commit hash>`. Replace it with the actual hash from Step 5.

Run:

```bash
COMMIT_HASH=$(git rev-parse --short HEAD)
echo "Backfilling with: $COMMIT_HASH"
```

Then use Edit on `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md`:

```
old_string: <TBD: filled in by Task 8 commit hash>
new_string: <COMMIT_HASH FROM ABOVE>
```

Then create a follow-up commit:

```bash
git add docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md
git commit -m ":memo: Backfill commit hash in superseded plan doc note"
```

- [ ] **Step 7: Verify final state**

Run:

```bash
git log --oneline -3
git show --stat HEAD~1
```

Expected:
- HEAD: ":memo: Backfill commit hash in superseded plan doc note"
- HEAD~1: ":memo: Revise AEAD prototype skeleton per minimal redesign" with 8 files changed
- HEAD~2: c063688f (the original prototype commit)

- [ ] **Step 8: Do NOT push**

Per `feedback_stop_at_push.md`: stop at local commit. Report the two new commit hashes to the user.

---

## Self-Review (executed by plan author after writing this plan)

**1. Spec coverage** (against `docs/superpowers/specs/2026-04-26-aead-minimal-redesign.md` "Implementation impact (libpna)" section):

- "Remove AeadContext's archive and entry field" → Task 1 ✅ (struct rewritten with only key, encryption_byte, mode_byte, entry_index)
- "Remove build_aad of length 86; replace with 11-byte builder" → Task 1 ✅
- "Remove build_nonce derivation logic; replace with CSPRNG" → Task 1 ✅ (replaced with `generate_nonce`)
- "Remove HKDF references" → Task 1 ✅ (module doc) + Task 2 ✅ (README)
- "Skip AENC/FENC chunk type definitions in lib/src/chunk/types.rs" → Out of scope for this plan: the prototype skeleton does not currently define AENC/FENC chunk types (the README's activation step #5 mentioned it but that step is being removed in Task 2). When the prototype is activated, no AENC/FENC step will exist to skip.
- "The dependency list shrinks: hkdf = 0.13 is no longer needed" → Task 2 ✅ (removed from README's Cargo.toml step)

**2. Placeholder scan:** One intentional placeholder in Task 7 Step 2 (`<TBD: filled in by Task 8 commit hash>`) — this is filled in by Task 8 Step 6 as part of the executable plan flow. Not a plan failure.

**3. Type/method consistency:**
- `AeadContext` struct has fields `key`, `encryption_byte`, `mode_byte`, `entry_index` — used identically in `build_aad` and in test `dummy_context()`. ✅
- `AEAD_AAD_LEN` constant (= 11) used in `build_aad` array size and in tests. ✅
- `AEAD_NONCE_LEN` constant (= 12) used in `generate_nonce` array size and in tests. ✅
- `generate_nonce` function signature `<R: rand_core::RngCore + rand_core::CryptoRng>(rng: &mut R) -> [u8; AEAD_NONCE_LEN]` consistent with how it's referenced from `aes_gcm.rs` doc comments (`super::generate_nonce(rng)`). ✅
- `Final-chunk flag` (capitalized) used consistently in all comments matching the spec's `Final-chunk flag` naming (vs. the old `is_final_chunk`). ✅

**4. Commit boundary:** Two commits in Task 8 — one for the main revision, one short follow-up for the commit-hash backfill. The backfill is split because we cannot embed a commit hash in the same commit that produces it.

---

## Out-of-scope

The following are deferred to separate plans / activations (per `feedback_independent_entities.md`):

- **Actual AEAD activation** (declaring `mod aead;` in `lib/src/cipher.rs`, adding `aes-gcm` / `camellia` / `subtle` / `zeroize` deps, implementing `encrypt_chunk` / `decrypt_chunk` bodies, wiring `CipherWriter::GcmAes` and `DecryptReader` counterparts, defining the cipher writer enum variants, plumbing PHSF → AeadContext::key). This is the next phase, not part of this revision plan.
- **Spec or libpna PR creation / push to `origin`**. This plan stops at local commit; pushing requires explicit user instruction.
- **External crypto audit** (1–3 month engagement). Required before the AEAD path is declared stable; not blocked by skeleton revisions.
