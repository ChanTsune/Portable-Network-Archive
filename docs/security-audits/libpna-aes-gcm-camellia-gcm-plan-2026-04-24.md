# libpna AEAD Migration Plan: AES-256-GCM + Camellia-256-GCM (Phase 1)

> **Status**: Design discussion outcome, pre-implementation. Captures decisions from the 2026-04-23/24 design session.
> **Action**: Roadmap for Phase 1 AEAD introduction. Phase 2 (ChaCha20-Poly1305 etc.) deferred.

## Plan Metadata

| Field | Value |
|---|---|
| Plan date | 2026-04-24 |
| Plan author | Claude Opus (manager) + 3 Opus subagents (industry survey / streaming AEAD / migration cases) |
| Origin finding | CRY-001 in `libpna-2026-04-23.md` (Critical: unauthenticated encryption) |
| Phase 1 scope | **AES-256-GCM + Camellia-256-GCM** (DIY in-tree implementation for Camellia) |
| Phase 2 (deferred) | ChaCha20-Poly1305, AES-OCB, AEAD registry generalization |
| Format break | Yes — new chunk types + AHED major bump |
| Estimated effort | 8–12 weeks (excluding external audit, 1–3 months additional) |

---

# Decision Log (2026-04-23/24 session)

| Decision | Outcome | Rationale |
|---|---|---|
| **Phase 1 cipher scope** | AES-256-GCM + Camellia-256-GCM only | Maintains existing PNA AES+Camellia symmetry; aligns with Japan CRYPTREC dual-cipher policy |
| **ChaCha20-Poly1305** | Deferred to Phase 2 | Reduces audit surface; allows clean (encryption, mode) tuple semantics in Phase 1 |
| **Spec design pattern** | **Option B** (mode_byte = 2 for GCM) | Preserves NIST taxonomy; (encryption, mode) 2-tuple intact; minimal spec diff; user-intuitive |
| **Camellia-GCM impl strategy** | **DIY in-tree** (libpna 内実装) | No `camellia-gcm` crate exists; ghash + camellia + ctr 既存クレート組み合わせ可能 |
| **Default chunk size** | 64 KiB | age-compatible; sweet spot per Tink/age analysis; bounded streaming memory |
| **KDF parameters** | Argon2id m=64MiB t=3 p=4 (CRY-005 fix 統合) | RFC 9106 first-recommended profile |
| **AAD scope** | Full AAD (algorithm + salts + indices + final flag + FHED hash) | Prevents downgrade (CRY-003), Kohno 2004 metadata tampering |
| **Per-entry HKDF subkey** | Yes (HKDF-SHA-256 from master key) | Avoids GCM 2^32 invocations/key cap; clean key separation |
| **Backward compat** | Side-by-side; old reader rejects via AHED major bump | LUKS / OpenSSH 流 staged rollout |
| **Migration tool** | `pna migrate --upgrade-aead` (extends recently-stabilized `pna migrate`) | Borg `transfer` precedent |

---

# Spec-Level Changes (Option B: NIST Taxonomy Maintained)

## Cipher Algorithm Registry — Unchanged

```
Slot 0 = Rijndael (AES) — existing, FHED encryption byte = 1
Slot 1 = Camellia       — existing, FHED encryption byte = 2
```

No new algorithm slot needed. AES-256 and Camellia-256 both keep encryption byte 1 and 2.

## Cipher Mode Registry — Extended

```
Slot 0 = CBC    — existing, FHED mode byte = 0
Slot 1 = CTR    — existing, FHED mode byte = 1
Slot 2 = GCM    — NEW,      FHED mode byte = 2  ← Phase 1 addition
```

Valid (encryption, mode) tuple matrix:

|  | CBC (0) | CTR (1) | **GCM (2)** |
|---|---|---|---|
| **AES (1)** | ✅ legacy | ✅ legacy | 🆕 **AES-256-GCM** |
| **Camellia (2)** | ✅ legacy | ✅ legacy | 🆕 **Camellia-256-GCM** |

Clean 2×3 matrix, fully symmetric, NIST taxonomy compliant.

## New Chunk Types

| Chunk | Critical? | Per-archive count | Role |
|---|---|---|---|
| **`PSLT`** (Per-archive SaLT) | Yes (uppercase first letter) | 1 (after AHED) | 64-bit nonce salt + KDF salt scope |
| **`PESL`** (Per-Entry SaLT) | Yes | 1 per encrypted entry (after FHED) | Per-entry HKDF salt (entry_id) |

### PSLT chunk format
```
4 bytes: length (= 16)
4 bytes: type "PSLT"
8 bytes: nonce_salt (random, CSPRNG)
8 bytes: kdf_salt   (random, CSPRNG)
4 bytes: CRC32
```

### PESL chunk format
```
4 bytes: length (= 8)
4 bytes: type "PESL"
8 bytes: entry_salt (random, CSPRNG, per-entry)
4 bytes: CRC32
```

## FHED Mode Byte Extension

Existing FHED layout preserved. Only mode byte semantics extended:
- `mode = 2` (GCM) requires the entry to follow GCM data layout (see below).
- Old readers seeing `mode = 2` MUST fail with "Unknown cipher mode" (PNG ancillary chunk semantics).

## FDAT Data Layout under GCM

```
Per-chunk FDAT data (under AEAD GCM):
  bytes [0 .. N-16]  : ciphertext (variable length)
  bytes [N-16 .. N]  : GCM authentication tag (16 bytes, fixed)
```

The chunk-level CRC32 (already present per chunk header) covers the entire `(ciphertext || tag)` for transport-error detection. Tag-based authenticity is independent and stronger.

## Solid Mode (SDAT) under GCM

Same construction as FDAT. Each SDAT carries `(ciphertext || tag)`. The SHED → SDAT* → SEND sequence forms one logical encryption stream.

## AHED Version Bump

Current AHED carries `(major, minor, archive_number, ...)`. Bump:
- Old: `major = 0`
- **AEAD-capable: `major = 1`** (signals new chunk types + AEAD reader requirement)

Old readers (major=0 only) MUST refuse to process major=1 archives.

## Nonce Construction

```
nonce[0..8]   = PSLT.nonce_salt          (per-archive, random)
nonce[8..12]  = chunk_index (u32 BE)     (per-entry chunk counter, starts at 0)
```

Per-entry chunk_index resets when a new entry begins. Combined with per-entry HKDF subkey, this guarantees nonce uniqueness within (key, nonce) pairs even across many entries.

## AAD (Associated Data) Construction

```
AAD = concat(
  "PNA-v1-AEAD"               // 11 bytes — version magic
  encryption_byte             // 1 byte  (1 = AES, 2 = Camellia)
  mode_byte                   // 1 byte  (= 2, GCM)
  nonce_salt (PSLT)           // 8 bytes
  kdf_salt   (PSLT)           // 8 bytes
  entry_salt (PESL)           // 8 bytes
  entry_index (u32 BE)        // 4 bytes — archive 内 entry 順序
  chunk_index (u32 BE)        // 4 bytes — entry 内 chunk 順序
  is_final_chunk              // 1 byte  (0x01 if last FDAT before FEND, else 0x00)
  fhed_metadata_hash          // 32 bytes — SHA-256(FHED + all metadata chunks)
)
```

This AAD design defends against:
- **CRY-003 (algorithm downgrade)**: encryption_byte / mode_byte tampering → AAD mismatch → tag fail
- **Truncation attack**: is_final_chunk tampering → tag fail
- **Reordering attack**: chunk_index tampering → tag fail
- **Cross-archive substitution**: nonce_salt mismatch → tag fail (also MUL-002 fix)
- **Metadata tampering (Kohno 2004)**: fhed_metadata_hash mismatch → tag fail

## Per-Entry Key Derivation (HKDF)

```
master_key = Argon2id(
    password,
    salt = PSLT.kdf_salt,
    m_cost = 65536 KiB (64 MiB),    # CRY-005 fix: RFC 9106 first-recommended
    t_cost = 3,
    p_cost = available_parallelism()
)

per_entry_key = HKDF-SHA256(
    ikm  = master_key,
    salt = PESL.entry_salt,
    info = "PNA-v1-AEAD-" || encryption_name || "-256-GCM"
)
```

The `info` parameter binds the algorithm name into key derivation. An attacker who tampers with the algorithm declaration produces a different key → tag verification fails → archive rejected.

`encryption_name` = `"AES"` for encryption_byte=1, `"Camellia"` for encryption_byte=2.

## Multipart Considerations

`PSLT.nonce_salt` is shared across all parts of a multipart archive. Each part MUST carry an identical PSLT chunk after its AHED. Mismatch → reject (also MUL-002 fix).

Per-entry chunk_index continues across parts if an entry spans an ANXT boundary.

---

# Implementation-Level Changes (libpna)

## File Plan

```
lib/Cargo.toml:
  + aes-gcm = "0.10"        # AES-GCM (audited NCC Group 2020)
  + ghash = "0.6"           # GCM の Galois hash (Camellia-GCM DIY 用)
  + hkdf = "0.13"           # subkey derivation
  + zeroize = "1.7"         # CRY-004 同時 fix
  + subtle = "2.5"          # constant-time tag 比較
  # (camellia, ctr は既に存在)

lib/src/cipher.rs:
  CipherWriter enum に GcmAes, GcmCamellia variant 追加
  DecryptReader 側も対称的に追加

lib/src/cipher/aead.rs (新規, ~150 行):
  AeadContext struct, AAD 構築 helper, nonce 派生 helper

lib/src/cipher/aead/aes_gcm.rs (新規, ~80 行):
  aes-gcm crate のラッパ (Aes256Gcm 用 trait 実装)

lib/src/cipher/aead/camellia_gcm.rs (新規, ~300-400 行):
  DIY 実装。NIST SP 800-38D 準拠で camellia + ctr + ghash 合成
  公開 API は aes-gcm と同じ AeadCore trait 実装

lib/src/cipher/aead/read.rs (新規, ~120 行):
  AeadReader<R>: chunk 単位で tag verify

lib/src/cipher/aead/write.rs (新規, ~120 行):
  AeadWriter<W>: chunk 単位で encrypt + tag append

lib/src/entry/options.rs:
  CipherMode::GCM 追加

lib/src/entry/write.rs:
  AEAD path: PSLT/PESL emit, master_key + per_entry_key derivation, AAD 構築

lib/src/entry/read.rs:
  AEAD path: PSLT/PESL parse, key derivation, tag verify

lib/src/chunk/types.rs:
  PSLT, PESL chunk type 定義

lib/src/archive/header.rs:
  AHED major bump 対応 (major=1 で AEAD-capable signal)

lib/src/archive/write.rs:
  PSLT chunk emit

lib/src/hash.rs:
  HKDF helper (hkdf crate のラッパ)
```

**合計**: 新規 module 6 + 既存 9 ファイル変更 = **約 1,500-2,000 行追加**

## Camellia-GCM DIY Implementation (per NIST SP 800-38D)

### Core algorithm

```rust
use camellia::Camellia256;
use ctr::Ctr32BE;
use ghash::GHash;

pub(crate) struct CamelliaGcm256 {
    cipher: Camellia256,
    ghash_key: GHash,
}

impl CamelliaGcm256 {
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Camellia256::new(key.into());
        // H = E(K, 0^128)
        let mut h = [0u8; 16];
        cipher.encrypt_block((&mut h).into());
        let ghash_key = GHash::new(&h.into());
        Self { cipher, ghash_key }
    }

    pub fn encrypt(&self, nonce: &[u8; 12], aad: &[u8], plaintext: &mut [u8])
        -> [u8; 16]
    {
        // J0 = nonce || 0^31 || 1
        let mut j0 = [0u8; 16];
        j0[..12].copy_from_slice(nonce);
        j0[15] = 1;

        // C = CTR_K(plaintext, J0+1, ...)
        let mut ctr = Ctr32BE::<Camellia256>::from_core(...);
        ctr.apply_keystream(plaintext);

        // S = GHASH(H, AAD || pad || C || pad || [len(A)]_64 || [len(C)]_64)
        let mut hasher = self.ghash_key.clone();
        hasher.update_padded(aad);
        hasher.update_padded(plaintext);
        let mut len_block = [0u8; 16];
        len_block[..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
        len_block[8..].copy_from_slice(&((plaintext.len() as u64) * 8).to_be_bytes());
        hasher.update(&[len_block.into()]);
        let s = hasher.finalize();

        // T = MSB_128(GCTR(K, J0, S))
        let mut tag_block = j0;
        self.cipher.encrypt_block((&mut tag_block).into());
        let mut tag = [0u8; 16];
        for i in 0..16 { tag[i] = tag_block[i] ^ s[i]; }
        tag
    }

    pub fn decrypt(&self, nonce: &[u8; 12], aad: &[u8], ciphertext: &mut [u8],
                   tag: &[u8; 16]) -> Result<(), TagError> {
        // 1. Recompute expected tag (same as encrypt's GHASH path)
        // 2. subtle::ConstantTimeEq で expected_tag == tag を比較
        // 3. 一致したら CTR で復号、不一致なら Err (RUP 防御: plaintext は touch しない)
    }
}
```

### Critical implementation pitfalls

| Risk | Mitigation |
|---|---|
| **Counter endianness** (GCM uses 32-bit BE) | `Ctr32BE` (NOT `Ctr64BE` / `Ctr128BE`) を選ぶ |
| **GHASH input padding** (16-byte boundary, zero-pad) | `ghash::GHash::update_padded` を使う、自分で padding しない |
| **Length field encoding** (bit length, not byte length) | `(len * 8).to_be_bytes()` |
| **Tag comparison timing attack** | `subtle::ConstantTimeEq` で比較 |
| **RUP (Releasing Unverified Plaintext)** | tag verify **前** に plaintext を返さない |
| **Nonce reuse detection** | API レベルで session 内 counter 管理、reuse は API で禁止 |
| **Zero-length plaintext** | edge case test 必須 |
| **Max plaintext length** (2^39 - 256 bits) | runtime check |

## Test Plan

```
lib/tests/aead/                         # 新規 test directory
  aes_gcm_round_trip.rs                 # round-trip
  aes_gcm_nist_cavp.rs                  # NIST CAVP test vectors
  aes_gcm_openssl_compat.rs             # OpenSSL CLI 出力との bit-exact 比較
  camellia_gcm_round_trip.rs
  camellia_gcm_openssl_compat.rs        # OpenSSL CLI 出力との bit-exact 比較 (Camellia-GCM)
  tag_tampering.rs                      # bit-flip → InvalidData エラー確認
  truncation_detection.rs               # final chunk 削除 → エラー
  reordering_detection.rs               # chunk 順序入替 → エラー
  algorithm_downgrade.rs                # encryption byte 改竄 → エラー
  cross_archive_substitution.rs         # 別 archive の FDAT 置換 → エラー
  multipart_consistency.rs              # 各 part の PSLT 不一致 → エラー
  solid_mode_aead.rs                    # SHED-SDAT-SEND の AEAD round-trip
  backward_compat_read.rs               # 旧 CBC/CTR archive を読めること
  forward_compat_refuse.rs              # 旧 reader が新 archive を reject

fuzz/fuzz_targets/
  aead_decrypt_aes_gcm.rs
  aead_decrypt_camellia_gcm.rs
  aead_archive_parse.rs
```

## Test Vector Sourcing for Camellia-GCM

No NIST CAVP exists for Camellia-GCM (NIST tests AES only). Recommended sources:

1. **OpenSSL CLI**: `openssl enc -camellia-256-gcm -K <hex> -iv <hex> -e -in plain -out cipher` → fixture
2. **BoringSSL** crypto/cipher_extra/test/ — verify if Camellia-GCM included
3. **BouncyCastle** Java reference vectors
4. **Cross-implementation triangulation**: 同じ (key, nonce, plaintext, AAD) を 3 実装に投入し全一致を要求

PNA's CI MUST include OpenSSL CLI cross-validation for every release.

---

# Backward Compatibility & Rollout Strategy

## Reader Behavior Matrix

| Reader version | Sees old archive (major=0) | Sees new archive (major=1) |
|---|---|---|
| Old (≤ v0.33) | ✅ reads | ❌ refuse ("Unsupported archive version") |
| New (v0.34+) | ✅ reads | ✅ reads |

## Writer Default Behavior Roadmap (LUKS / OpenSSH 流)

| Version | Writer default | Notes |
|---|---|---|
| **v0.34** (intro) | 旧 (CBC/CTR) | AEAD support added; opt-in via `--aead` |
| **v0.40** (default flip) | **AEAD (AES-256-GCM)** | Notice in release notes: "future versions will refuse to write CBC/CTR" |
| **v1.0** (mark deprecated) | AEAD; `--legacy-cipher` 必須 for CBC/CTR write | First stable release |
| **v2.0** (write refuse) | AEAD only | Reading legacy archives still works |
| **v3.0+** | AEAD only | Read of legacy archives MAY emit warning escalation |

Read-only support for legacy archives is **maintained indefinitely** (LUKS1 precedent).

## Migration Tool Extension

Extend the recently-stabilized `pna migrate` (commit b4a02edb on 2026-04-23):

```bash
pna migrate --upgrade-aead --in old.pna --out new.pna [--password]
```

- Read old archive (CBC/CTR), decrypt
- Re-encrypt under AEAD scheme with same password (or new password via `--re-key`)
- Preserve all metadata
- Output is AEAD-only archive

This connects the migrate stabilization work to the AEAD migration arc.

---

# Audit Plan (DIY Crypto Mandatory)

| Phase | Audit target | Reviewer |
|---|---|---|
| **Self-audit** | Code review with NIST SP 800-38D open, line-by-line | maintainer |
| **MIRI** | UB detection (especially around any `unsafe`) | CI 自動 |
| **Cross-implementation** | OpenSSL/BoringSSL/BouncyCastle 出力 と bit-exact 一致 | CI 自動 |
| **Static analysis** | `cargo audit`, `cargo geiger` | CI 自動 |
| **External crypto review** | 専門家 (Filippo Valsorda, Trail of Bits, NCC Group, Cure53) | 有償 external |

**External review なしの release は推奨しない**。DIY crypto が史上最も多くの CVE を生んだカテゴリ。

Recommended reviewers (alphabetical):
- **Cure53** (https://cure53.de/) — Berlin-based, did rsync/age reviews
- **NCC Group Cryptography Services** — did RustCrypto AEADs review (2020)
- **Trail of Bits** — did multiple Rust crypto reviews (rustls, aws-lc-rs)
- **Filippo Valsorda** independent — did restic crypto review, knows STREAM construction intimately

---

# Updated Effort Estimate

| Phase | Effort |
|---|---|
| Spec doc drafting (this doc + spec repo PR) | 1-2 weeks |
| Spec community review | 2-4 weeks (PR comment iteration) |
| libpna implementation: AES-GCM | 1-2 weeks |
| libpna implementation: Camellia-GCM (DIY) | 1 week (with reference) |
| Test additions (round-trip + negative cases) | 1 week |
| Camellia-GCM cross-validation fixtures (OpenSSL CLI based) | 3-5 days |
| Fuzz harness | 2-3 days |
| Migration tool extension (`pna migrate --upgrade-aead`) | 2-3 days |
| Backward compat implementation (multi-version reader) | 3-5 days |
| **External crypto audit** | 1-3 months (external dependency) |
| **Subtotal (excluding audit)** | **8-12 weeks** |

---

# Open Questions / Decisions Pending

| Q | Status | Note |
|---|---|---|
| Q1: Cipher scope (Phase 1) | ✅ **DECIDED**: AES-GCM + Camellia-GCM | 2026-04-24 |
| Q2: Spec design (Option A vs B) | ✅ **DECIDED**: Option B (mode byte = 2) | 2026-04-24 |
| Q3: Camellia-GCM impl strategy | ✅ **DECIDED**: DIY in-tree | 2026-04-24 |
| Q4: AHED v0 → v1 bump details | 🔍 spec drafting で詳細化 | Step B |
| Q5: PSLT/PESL chunk binary layout | 🔍 spec drafting で詳細化 | Step B |
| Q6: AAD exact byte layout | 🔍 spec drafting で詳細化 | Step B |
| Q7: Migration tool UX (`pna migrate --upgrade-aead`) | 後日 (CLI 設計時) | Phase 2 |
| Q8: External audit vendor 選定 | 未着手 | 実装着手後 |
| Q9: spec major bump 対象範囲 (CRY-001 のみ vs MUL-002 等も統合) | 推奨: 統合 (1 回の bump で複数 finding 解決) | spec drafting で確定 |
| Q10: AHED に "AEAD-required" flag を追加するか | 推奨: 追加 (downgrade 防御 defense-in-depth) | spec drafting で確定 |

---

# Phase 2 (Deferred)

Items intentionally deferred to a future cycle:
- **ChaCha20-Poly1305** support (Cargo dep + new encryption byte 3 + flat AEAD identifier transition)
- **AES-OCB** support (RustCrypto crate 不在、要 DIY or wait)
- **AEGIS / Ascon** (post-quantum era prep)
- **Cipher suite registry pattern** (TLS 1.3 流の flat namespace 移行)
- **Multi-recipient AEAD** (key wrapping for asymmetric recipients)
- **Key commitment** (Soatok 2024 Invisible Salamanders 防御)

---

# Reference Documents (to be created in Step D)

- `docs/security-audits/libpna-aead-implementation-reference-2026-04-24.md` (Step D agent output)
  - NIST SP 800-38D pseudocode
  - RFC 6367 details
  - Camellia-GCM test vector sources
  - RustCrypto aes-gcm code reference
  - GCM implementation pitfalls catalog
  - AAD design best practices

---

# Connection to Existing Audit Findings

This plan addresses the following findings from `libpna-2026-04-23.md`:

| Finding | Addressed by |
|---|---|
| **CRY-001** (no AEAD/MAC) | Phase 1 core scope |
| **CRY-002** (CBC padding oracle) | CBC mode マークドプリケート、新規 default GCM (writers) で oracle 不在 |
| **CRY-003** (algorithm downgrade) | AAD に encryption_byte + mode_byte + HKDF info を bind |
| **CRY-005** (Argon2id default 弱い) | Phase 1 同時 fix (KDF parameter 強化) |
| **CRY-007** (RNG failure 検知なし) | Phase 1 同時 fix (HKDF subkey 派生 + try_fill_bytes) |
| **CRY-010** (constant-time 比較なし) | Phase 1 で `subtle` 依存追加 |
| **MUL-002** (no archive-set ID) | PSLT.nonce_salt がそのまま archive-set ID |
| **MUL-005** (AHED 不整合) | PSLT 一致 check で同時 fix |

Phase 1 spec major bump で 8 件の findings を一括解消。これが「spec change を統合する」戦略の根拠 (Q9)。

---

*End of Plan.*
