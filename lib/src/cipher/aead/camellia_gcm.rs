//! Camellia-256-GCM AEAD cipher (DIY in-tree implementation).
//!
//! **Status: prototype skeleton (2026-04-24).**
//!
//! No `camellia-gcm` crate exists in the RustCrypto ecosystem (verified
//! 2026-04-24 via `cargo search`). This module composes Camellia-256-GCM from
//! existing audited primitives:
//!
//! - `camellia 0.2.0` — block cipher (NTT/Mitsubishi reference, RFC 3713)
//! - `ctr 0.10.0` — counter mode (already a libpna dependency)
//! - `ghash 0.6.0` — Galois hash (the MAC component of GCM)
//!
//! The construction follows NIST SP 800-38D Algorithm 4 (GCM-AE) and
//! Algorithm 5 (GCM-AD), with the underlying block cipher being Camellia-256
//! per RFC 6367 §3.
//!
//! # Implementation TODO
//!
//! 1. Add `ghash = "0.6"` to `lib/Cargo.toml` (camellia and ctr already present).
//! 2. Implement `CamelliaGcm256::new`, `encrypt`, `decrypt` per NIST SP 800-38D.
//! 3. Sourcing test vectors:
//!    - Generate via `openssl enc -camellia-256-gcm` (RFC 6367-compliant impl).
//!    - Cross-validate against BoringSSL / BouncyCastle if available.
//!    - File fixtures under `lib/tests/aead/fixtures/camellia_gcm/`.
//! 4. Add `lib/tests/aead/camellia_gcm_openssl_compat.rs` for bit-exact verification.
//! 5. **External crypto audit MANDATORY** before stable release (DIY crypto).
//!
//! # Implementation pitfalls (must avoid)
//!
//! Per `docs/security-audits/libpna-aes-gcm-camellia-gcm-plan-2026-04-24.md` §"Hard parts":
//!
//! - **Counter endianness**: GCM uses 32-bit BE counter (use `Ctr32BE`, NOT `Ctr64BE` / `Ctr128BE`).
//! - **GHASH input padding**: AAD and ciphertext zero-padded to 16-byte boundary; use `GHash::update_padded`, never roll padding manually.
//! - **Length field encoding**: `[len(A)]_64 || [len(C)]_64` is bit length, not byte length: use `(len * 8).to_be_bytes()`.
//! - **Tag comparison**: MUST be constant-time via `subtle::ConstantTimeEq`, never `==`.
//! - **RUP (Releasing Unverified Plaintext)**: tag verification MUST precede plaintext release.
//! - **Nonce reuse detection**: enforce monotonic counter at API level; encrypt MUST refuse nonce reuse.

#![allow(dead_code)] // skeleton

use super::{AeadContext, AuthFailure};

/// Camellia-256-GCM cipher state.
///
/// Constructed once per per-entry derived key. Reused for every chunk of that entry.
///
/// Internal layout (planned):
/// ```text
/// struct CamelliaGcm256 {
///     cipher: Camellia256,                  // for E(K, ...) operations
///     ghash_key: GHash,                     // initialized with H = E(K, 0^128)
/// }
/// ```
pub(crate) struct CamelliaGcm256 {
    // Skeleton — actual fields TBD on implementation.
    _phantom: core::marker::PhantomData<()>,
}

impl CamelliaGcm256 {
    /// Create a new Camellia-256-GCM cipher state from a 32-byte key.
    ///
    /// Per NIST SP 800-38D §6.3:
    /// ```text
    /// H = E(K, 0^128)   // hash subkey for GHASH
    /// ```
    ///
    /// **Not yet implemented.**
    pub(crate) fn new(_key: &[u8; 32]) -> Self {
        unimplemented!("Camellia-256-GCM not yet implemented; see README.md")
    }
}

/// Encrypt a single chunk under Camellia-256-GCM.
///
/// Returns ciphertext (same length as plaintext) appended with the 16-byte tag.
///
/// **Not yet implemented.**
pub(crate) fn encrypt_chunk(
    _ctx: &AeadContext,
    _chunk_index: u32,
    _is_final: bool,
    _plaintext: &[u8],
) -> Vec<u8> {
    unimplemented!("Camellia-256-GCM encryption not yet implemented")
}

/// Decrypt and verify a single chunk under Camellia-256-GCM.
///
/// **Not yet implemented.**
pub(crate) fn decrypt_chunk(
    _ctx: &AeadContext,
    _chunk_index: u32,
    _is_final: bool,
    _chunk_data: &[u8],
) -> Result<Vec<u8>, AuthFailure> {
    unimplemented!("Camellia-256-GCM decryption not yet implemented")
}
