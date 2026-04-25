# Reference: AES-GCM and Camellia-GCM Implementation Details

> **Status**: Implementation reference document. Compiled 2026-04-24 by Opus subagent (D step in A→B→C→D plan). All facts traced to primary source. Items marked **未確認** could not be verified within the time budget and must be confirmed by the implementer before relying on them.

## 0. Document Conventions

Every quoted spec line is wrapped in ASCII quotes and followed by `[<doc> §<section>, p<page>]`. Unicode mathematical symbols from the source documents are reproduced as plain ASCII where unambiguous (e.g. `||` for concatenation, `^` for XOR, `·` for GF multiplication, `len(X)` for bit length). Where the source uses subscripts/superscripts, this document uses `_` and `^`.

## 🚨 CRITICAL FINDING — Reconsider "DIY" decision

**The Step D research revealed that the "DIY Camellia-GCM" assumption may be unnecessary.** The RustCrypto `aes-gcm` crate is structurally **generic over any 128-bit block cipher**, not AES-specific. A literal **3-line type alias** suffices:

```rust
use aes_gcm::AesGcm;
use aes_gcm::aead::generic_array::typenum::U12;
use camellia::Camellia256;

pub type Camellia256Gcm = AesGcm<Camellia256, U12>;
```

This works because `AesGcm<Aes, NonceSize, TagSize>` requires only `Aes: BlockSizeUser<BlockSize=U16> + BlockCipherEncrypt + KeyInit`, all of which `Camellia256` already implements. The crate name `AesGcm` is a misnomer; the implementation is generic GCM.

**Implications**:
- Camellia-GCM implementation effort: **1 week → ~1 hour**
- External audit scope: dramatically reduced (we use audited `aes-gcm` GCM machinery, not handrolled GHASH/GCTR)
- Audit cost: lower
- Code maintenance: lower

**Caveat**: depends on `aes-gcm` crate's public type. If the crate maintainers later split AES-only and generic-GCM into separate crates, libpna would need to fork the generic part. This is a real but small risk.

**Decision needed**: Stay with full DIY (this doc's NIST SP 800-38D pseudocode), or pivot to 3-line composition? See §4.8 below for full discussion.

---

## 1. NIST SP 800-38D Pseudocode (Authoritative GCM Spec)

Source: NIST Special Publication 800-38D, "Recommendation for Block Cipher Modes of Operation: Galois/Counter Mode (GCM) and GMAC" (Dworkin, November 2007). PDF: <https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf>. Mirrored at <https://csrc.nist.gov/pubs/sp/800/38/d/final>.

### 1.1 Notation Used by the Spec (§4.2.2, p6)

| Symbol | Meaning |
|---|---|
| `CIPH_K(X)` | Forward block cipher under key K, applied to block X |
| `GCTR_K(ICB, X)` | GCTR keystream output for key K, initial counter block ICB, on bit string X |
| `GHASH_H(X)` | GHASH under hash subkey H applied to bit string X |
| `inc_s(X)` | Increment rightmost s bits of X by 1 mod 2^s |
| `LSB_s(X)`, `MSB_s(X)` | s rightmost / leftmost bits of X |
| `[x]_s` | Binary representation of integer x as s-bit string, MSB on left, LSB on right |
| `0^s` | String of s zero bits |
| `X · Y` | Multiplication in GF(2^128) defined by Algorithm 1 |
| `X ^ Y` | Bitwise XOR |
| `X || Y` | Concatenation |

Block size **shall** be 128 bits; underlying block cipher key **shall** be at least 128 bits [§5.1, p7].

### 1.2 Length Restrictions (§5.2.1.1, p8)

```
- len(P) <= 2^39 - 256       (max plaintext bit length)
- len(A) <= 2^64 - 1         (max AAD bit length)
- 1 <= len(IV) <= 2^64 - 1   (IV bit length range)
```

In bytes:

| Field | Max bytes | Max bits |
|---|---|---|
| Plaintext P | (2^36) − 32 = 64 GiB − 32 | 2^39 − 256 |
| AAD A | (2^61) − 1 (~2 EiB) | 2^64 − 1 |
| IV | (2^61) − 1 | 2^64 − 1 |

> "For IVs, it is recommended that implementations restrict support to the length of 96 bits, to promote interoperability, efficiency, and simplicity of design." [§5.2.1.1, p8]

### 1.3 Approved Tag Lengths (§5.2.1.2, p9)

> "the bit length of the tag, denoted t, is a security parameter ... t may be any one of the following five values: 128, 120, 112, 104, or 96. For certain applications, t may be 64 or 32; guidance for the use of these two tag lengths ... is given in Appendix C." [§5.2.1.2, p9]

For PNA we recommend hard-coding **t = 128** (full tag). Anything below 96 imposes Appendix C invocation/packet-length restrictions.

### 1.4 inc_32 Function (§6.2, p11)

```
inc_s(X) = MSB_{len(X)-s}(X) || [int(LSB_s(X)) + 1 mod 2^s]_s
```

GCM specifically uses `s = 32`, hence the name `inc_32`. The increment is **big-endian**, on the **rightmost 4 bytes** of the 16-byte counter block. After 2^32 increments the counter wraps and silently collides with the J0-derived first counter.

### 1.5 GF(2^128) Multiplication — Algorithm 1 (§6.3, p11–12)

Reduction polynomial: `R = 11100001 || 0^120` (i.e. `0xE1` followed by 120 zero bits). The full reduction polynomial is `R || 1`, corresponding to the algebraic polynomial `x^128 + x^7 + x^2 + x + 1` per the bit-reflected "little-endian" convention used by GCM.

```
Algorithm 1: X · Y
Input: blocks X, Y
Output: block X · Y

1. Let x_0 x_1 ... x_127 denote the sequence of bits in X.
2. Let Z_0 = 0^128 and V_0 = Y.
3. For i = 0 to 127:
     Z_{i+1} = Z_i                      if x_i = 0
              Z_i ^ V_i                 if x_i = 1
     V_{i+1} = V_i >> 1                 if LSB_1(V_i) = 0
              (V_i >> 1) ^ R            if LSB_1(V_i) = 1
4. Return Z_128
```

This is the bit-by-bit textbook implementation; production code uses 4-bit / 8-bit / PCLMUL / AVX-GFNI tables. **Critical implementation gotcha**: the bit indexing convention is opposite to "natural" big-endian — `x_0` is the *most significant* bit of the first byte but the *coefficient of u^0* in the polynomial. This is the "bit reversal" pitfall that has caused multiple historical bugs.

### 1.6 GHASH — Algorithm 2 (§6.4, p12)

```
Algorithm 2: GHASH_H(X)
Prerequisites: block H (the hash subkey)
Input: bit string X with len(X) = 128m
Output: block GHASH_H(X)

1. Partition X = X_1 || X_2 || ... || X_m (each X_i is a 128-bit block).
2. Let Y_0 = 0^128.
3. For i = 1, ..., m:  Y_i = (Y_{i-1} ^ X_i) · H
4. Return Y_m.
```

Equivalent closed form: `GHASH_H(X) = X_1·H^m ^ X_2·H^{m-1} ^ ... ^ X_m·H`. **Input length must be a multiple of 128 bits**; padding is the responsibility of the caller (per Algorithm 4 step 4–5).

### 1.7 GCTR — Algorithm 3 (§6.5, p13)

```
Algorithm 3: GCTR_K(ICB, X)
Prerequisites: approved 128-bit block cipher CIPH; key K
Input: initial counter block ICB; bit string X of arbitrary length
Output: bit string Y of bit length len(X)

1. If X is empty, return empty.
2. Let n = ceil(len(X) / 128).
3. Partition X = X_1 || X_2 || ... || X_{n-1} || X_n*
   where X_1..X_{n-1} are full 128-bit blocks, X_n* is the (possibly partial) final block.
4. CB_1 = ICB.
5. For i = 2 to n: CB_i = inc_32(CB_{i-1}).
6. For i = 1 to n-1: Y_i = X_i ^ CIPH_K(CB_i).
7. Y_n* = X_n* ^ MSB_{len(X_n*)}(CIPH_K(CB_n)).
8. Y = Y_1 || ... || Y_{n-1} || Y_n*.
9. Return Y.
```

In Algorithm 4, GCTR is called with `inc_32(J0)` as ICB, so `CB_1 = inc_32(J0) = J0+1` (counter starts at 1, not 0; counter 0 is reserved for the tag mask).

### 1.8 GCM-AE — Algorithm 4 (§7.1, p15)

```
Algorithm 4: GCM-AE_K(IV, P, A)
Prerequisites: 128-bit block cipher CIPH; key K; supported tag length t
Input: IV, plaintext P, AAD A (each of supported length)
Output: ciphertext C, tag T

1. H = CIPH_K(0^128)
2. Define J_0 as follows:
     If len(IV) = 96:  J_0 = IV || 0^31 || 1
     Else:             s = 128 * ceil(len(IV)/128) - len(IV)
                       J_0 = GHASH_H(IV || 0^{s+64} || [len(IV)]_64)
3. C = GCTR_K(inc_32(J_0), P)
4. u = 128 * ceil(len(C)/128) - len(C)   ; ciphertext zero-padding bits
   v = 128 * ceil(len(A)/128) - len(A)   ; AAD zero-padding bits
5. S = GHASH_H(A || 0^v || C || 0^u || [len(A)]_64 || [len(C)]_64)
6. T = MSB_t(GCTR_K(J_0, S))
7. Return (C, T).
```

[§7.1, p15] Verbatim (notation simplified to ASCII).

### 1.9 GCM-AD — Algorithm 5 (§7.2, p16–17)

```
Algorithm 5: GCM-AD_K(IV, C, A, T)
Prerequisites: 128-bit block cipher CIPH; key K; supported tag length t
Input: IV, ciphertext C, AAD A, authentication tag T
Output: plaintext P, OR FAIL

1. If lengths of IV, A or C are not supported, or len(T) != t, return FAIL.
2. H = CIPH_K(0^128)
3. Define J_0 as in Algorithm 4 step 2.
4. P = GCTR_K(inc_32(J_0), C)
5. u, v computed as in Algorithm 4 step 4.
6. S = GHASH_H(A || 0^v || C || 0^u || [len(A)]_64 || [len(C)]_64)
7. T' = MSB_t(GCTR_K(J_0, S))
8. If T == T': return P
   Else: return FAIL
```

> "Equivalent sets of steps that produce the correct output are permitted. In particular, the verification of the tag may precede the computation of the plaintext." [§7.2, p17]

**Implementation note**: Step 8 comparison **must** be constant-time. Step-order swap (verify-then-decrypt) is the safer pattern for streaming since it avoids RUP exposure (see §5.6).

### 1.10 §8.3 Invocation Limits per Key

> "The total number of invocations of the authenticated encryption function shall not exceed 2^32, including all IV lengths and all instances of the authenticated encryption function with the given key." [§8.3, p21]

This 2^32 cap applies under either:
1. Deterministic construction with non-96-bit IVs, or
2. RBG-based construction (random IVs) of any length.

The **only** way to escape the 2^32 cap is to use **96-bit IVs generated by the deterministic construction** (i.e. fixed-field || invocation-counter), in which case the cap is `2^s` where s is the bit length of the invocation counter field [§8.3 last paragraph, p21].

### 1.11 Appendix B / C — Tag-Length Caveats

> "if n denotes the total number of blocks in the encoding ... of the ciphertext and AAD, then there is a method of constructing a 'targeted' ciphertext forgery that is expected to succeed with a probability of approximately n/2^t." [App. B, p26]

Appendix C tables (p29) explicitly bound packet sizes for short tags:
- For **t=32**: max packet 2^10 bytes ⇒ ≤2^11 invocations per key.
- For **t=64**: max packet 2^15 bytes ⇒ ≤2^32 invocations per key.

For PNA we recommend **t=128 always** to dodge both the targeted-forgery degradation and the §8.3 cap interaction.

---

## 2. RFC 6367 (Camellia in TLS — Camellia-GCM Cipher Suites)

Source: <https://datatracker.ietf.org/doc/html/rfc6367> "Addition of the Camellia Cipher Suites to Transport Layer Security (TLS)" (Kanno & Kanda, September 2011).

### 2.1 Cipher Suite Definitions (§2.2)

Twenty Camellia-GCM cipher suites are defined, including:

```
TLS_RSA_WITH_CAMELLIA_128_GCM_SHA256      = {0xC0, 0x7A}
TLS_RSA_WITH_CAMELLIA_256_GCM_SHA384      = {0xC0, 0x7B}
TLS_DHE_RSA_WITH_CAMELLIA_128_GCM_SHA256  = {0xC0, 0x7C}
TLS_DHE_RSA_WITH_CAMELLIA_256_GCM_SHA384  = {0xC0, 0x7D}
... (through 0xC0,0x8D)
```

### 2.2 Specification (§3.2)

> "These cipher suites use authenticated encryption with additional data algorithms AEAD_CAMELLIA_128_GCM and AEAD_CAMELLIA_256_GCM ... These algorithms are based on AEAD_AES_128_GCM and AEAD_AES_256_GCM as defined in [RFC5116] and use the Camellia block cipher [RFC3713] in place of AES." [RFC 6367 §3.2]

This RFC is the **only IETF reference** that names "Camellia-GCM" as a primitive. **It does not specify the algorithm itself** — it merely says "use AEAD_AES_*_GCM construction with the AES block cipher swapped for Camellia." Per RFC 5116 §5.1, that means:
- 96-bit nonce (`N_MIN = N_MAX = 12`)
- 128-bit tag
- Plaintext limit 2^36 − 31 bytes
- AAD limit 2^61 − 1 bytes

### 2.3 Test Vectors

> **None.** RFC 6367 provides zero test vectors. Confirmed by full-text scan.

### 2.4 IANA Registration

§5 reserves codepoints 0xC0,0x72 through 0xC0,0x8D. No "Camellia-GCM AEAD" entry was registered in the IANA AEAD Algorithms registry at the time of RFC 6367 publication. **未確認**: whether RFC 6367-equivalent AEAD identifiers were later added to <https://www.iana.org/assignments/aead-parameters/>.

### 2.5 Implication for PNA

RFC 6367 gives PNA legal/standards cover to claim "Camellia-256-GCM" as the construction name without further definition handwaving — the construction is the well-known AES-256-GCM template with Camellia-256 substituted.

---

## 3. Camellia-GCM Test Vector Sources — **CRITICAL FINDING**

Result: **no public, named, bit-exact "Camellia-GCM" test vector suite exists** that we could locate. Verification will require either generating fixtures from a reference implementation (recommended) or contributing vectors upstream.

### 3.1 Source Audit Summary

| Source | Camellia? | GCM mode? | **Camellia-GCM vectors?** | Evidence |
|---|---|---|---|---|
| **OpenSSL** (master) | Yes (CBC/CFB/CTR/ECB/OFB/CTS) | Yes (AES-only) | **No** | `git ls-tree`: `crypto/camellia/cmll_{cbc,cfb,ctr,ecb,ofb}.c` exist but **no `cmll_gcm.c`**; `test/recipes/30-test_evp_data/` has `evpciph_camellia.txt` and `evpciph_camellia_cts.txt` only — no GCM file. EVP provider `cipher_camellia.c` does not register a GCM mode. <https://github.com/openssl/openssl/tree/master/crypto/camellia> |
| **BoringSSL** | **No** (Camellia not implemented) | Yes (AES-only) | **No** | `git ls-tree` of master returns zero `camellia` matches. |
| **BouncyCastle Java** (`bc-java`) | Yes (`CamelliaEngine`, `CamelliaLightEngine`, `CamelliaWrapEngine`) | Yes (`GCMBlockCipher` is generic over `BlockCipher`) | **No published vectors** | `core/src/test/java/org/bouncycastle/crypto/test/CamelliaTest.java` (3.4 KB) contains zero "GCM" / "Gcm" / "AEAD" tokens. `GCMTest.java` tests AES only. The combination is **mechanically possible** (`new GCMBlockCipher(new CamelliaEngine())`) but is **not part of BC's tested surface**. |
| **Botan** | Yes (`src/lib/block/camellia/`) | Yes (`src/lib/modes/aead/gcm/`) | **No published vectors** | `src/tests/data/aead/gcm.vec` headers: `[AES-128/GCM]`, `[AES-192/GCM]`, `[AES-256/GCM]`, `[ARIA-128/GCM]`, `[ARIA-192/GCM]`, `[ARIA-256/GCM]`, `[SM4/GCM]` — **no Camellia entry**. Botan's API allows `AEAD_Mode::create("Camellia-256/GCM")` but compliance is untested in upstream CI. |
| **Crypto++** | Yes (`camellia.h`, `camellia.cpp`) | Yes (`gcm.h`, `gcm.cpp` — generic over `BlockCipher`) | **未確認 / unlikely** | Crypto++'s `TestVectors/gcm.txt` is AES-only (cross-checked via search; <https://github.com/weidai11/cryptopp/blob/master/TestVectors/gcm.txt>). Combination is mechanically `GCM<Camellia>::Encryption` per the template. |
| **CRYPTREC / NTT** Camellia spec page | Yes (Camellia ECB vectors) | No (no AEAD mode vectors published) | **No** | <https://info.isl.ntt.co.jp/crypt/eng/camellia/> contains specifications, single-block test vectors, and reference C / Java sources for Camellia *primitive* only. No GCM combination. |
| **NIST CAVP** | n/a (CAVP only certifies AES, TDES, SHA-2/3 modes) | AES-GCM only | **No** | CAVP's GCM Validation List is AES-bound. |
| **RFC 5288** (AES-GCM) | n/a | Yes | n/a | Zero vectors in RFC 5288 itself. |
| **RFC 6367** | Camellia-GCM cipher suites | Yes | **No** | Confirmed in §2 above. |

### 3.2 Recommended Approach: Generate Fixtures from Two Independent References, Cross-Check

Because no canonical vector set exists, do the following:

**Step 1**: Generate vectors from **Botan** (C++, well-audited):

```cpp
#include <botan/aead.h>
#include <botan/hex.h>
#include <iostream>
int main() {
  auto enc = Botan::AEAD_Mode::create("Camellia-256/GCM", Botan::Cipher_Dir::Encryption);
  std::vector<uint8_t> key(32, 0x00);
  std::vector<uint8_t> nonce(12, 0x00);
  std::vector<uint8_t> aad{};
  Botan::secure_vector<uint8_t> pt{};   // empty plaintext, zero key, zero nonce
  enc->set_key(key);
  enc->set_associated_data(aad.data(), aad.size());
  enc->start(nonce);
  enc->finish(pt);
  std::cout << "ct||tag = " << Botan::hex_encode(pt) << std::endl;
}
// build: c++ -std=c++20 camgcm.cpp -lbotan-3
```

**Step 2**: Cross-check with **Crypto++**:

```cpp
#include <cryptopp/camellia.h>
#include <cryptopp/gcm.h>
#include <cryptopp/hex.h>
#include <cryptopp/files.h>
using namespace CryptoPP;

byte key[32] = {0}, iv[12] = {0};
GCM<Camellia>::Encryption enc;
enc.SetKeyWithIV(key, sizeof key, iv, sizeof iv);

std::string ct;
AuthenticatedEncryptionFilter f(enc, new StringSink(ct), false, 16); // 16-byte tag
f.MessageEnd();
// hex-encode ct
```

**Step 3**: If Botan and Crypto++ agree bit-exactly on at least 10 vectors covering edge cases (empty PT/empty AAD, only AAD, only PT, partial-block PT, multi-block PT, max-size cases), that pair becomes PNA's reference oracle. Encode vectors in JSON or KAT-style files in `lib/tests/test-vectors/camellia_256_gcm.json` and have CI assert exact equality.

**Step 4 (recommended)**: After PNA ships, **upstream the vectors** to either Botan or BouncyCastle so the wider ecosystem benefits. (This is the same path Crypto++'s ARIA-GCM vectors took.)

> Important: do **not** generate vectors from your own implementation and then test against them — you'd just be testing self-consistency. A generated test must come from an *independent* implementation at least once.

### 3.3 Why an OpenSSL CLI Approach Will NOT Work

A naive plan — `openssl enc -camellia-256-gcm` — **will fail**. OpenSSL has never registered Camellia-GCM as an EVP_CIPHER. Confirmed in §3.1 by absence of `EVP_camellia_*_gcm` symbols and no `cmll_gcm.c`. Don't waste fixture-generation time on this path.

### 3.4 Sanity Test Vector — AES-256-GCM Cross-Reference

For sanity-checking the GHASH/GCTR pipeline before swapping AES → Camellia, use NIST GCM Test Case 13 (AES-256-GCM, all-zero key, all-zero IV, empty PT, empty AAD), McGrew & Viega:

```
K  = 00000000000000000000000000000000 00000000000000000000000000000000
IV = 000000000000000000000000
P  = (empty)
A  = (empty)
C  = (empty)
T  = 530f8afbc74536b9a963b4f1c4cb738b
```

Source: McGrew & Viega "GCM revised spec" referenced in NIST SP 800-38D Appendix E item [6]; mirrored in <https://github.com/RustCrypto/AEADs/blob/master/aes-gcm/tests/aes256gcm.rs>. Use this to verify your GCTR/GHASH plumbing before substituting Camellia.

---

## 4. RustCrypto `aes-gcm` Code Reference

Source: <https://github.com/RustCrypto/AEADs/tree/master/aes-gcm> at master branch as of 2026-04-24.

### 4.1 Crate Manifest (`aes-gcm/Cargo.toml`)

```toml
[package]
name = "aes-gcm"
version = "0.11.0-rc.3"
edition = "2024"
rust-version = "1.85"

[dependencies]
aead = { version = "0.6.0-rc.10", default-features = false }
cipher = "0.5"
ctr = "0.10"
ghash = { version = "0.6", default-features = false }
subtle = { version = "2", default-features = false }

# optional
aes = { version = "0.9", optional = true }
zeroize = { version = "1", optional = true, default-features = false }
```

The architecture is **strictly compositional**: `aes-gcm` depends only on the abstract `cipher` traits, the `ghash` universal hash, and the `ctr` mode. **AES is an optional feature**. The crate does not contain any AES code itself.

### 4.2 The `AesGcm` Struct (`aes-gcm/src/lib.rs`)

```rust
pub const A_MAX: u64 = (1 << 61) - 1;     // matches NIST 2^64-1 bits / 8
pub const P_MAX: u64 = (1 << 36) - 32;    // matches NIST 2^39-256 bits / 8

#[derive(Clone)]
pub struct AesGcm<Aes, NonceSize, TagSize = U16>
where
    TagSize: self::TagSize,
{
    cipher: Aes,
    ghash: GHash,
    nonce_size: PhantomData<NonceSize>,
    tag_size: PhantomData<TagSize>,
}

pub type Aes128Gcm = AesGcm<Aes128, U12>;
pub type Aes256Gcm = AesGcm<Aes256, U12>;
```

Notice: `AesGcm` is **misnamed**. The generic `Aes` parameter accepts **any** type implementing `BlockSizeUser<BlockSize = U16> + BlockCipherEncrypt + KeyInit`. Renaming it `Gcm<Cipher, ...>` would be more accurate.

### 4.3 Trait Bounds

```rust
impl<Aes, NonceSize, TagSize> KeyInit for AesGcm<Aes, NonceSize, TagSize>
where
    Aes: BlockSizeUser<BlockSize = U16> + BlockCipherEncrypt + KeyInit,
    TagSize: self::TagSize,
{
    fn new(key: &Key<Self>) -> Self { Aes::new(key).into() }
}

impl<Aes, NonceSize, TagSize> AeadInOut for AesGcm<Aes, NonceSize, TagSize>
where
    Aes: BlockSizeUser<BlockSize = U16> + BlockCipherEncrypt,
    NonceSize: ArraySize,
    TagSize: self::TagSize,
{ ... }
```

The `TagSize` is a sealed trait restricting valid sizes to `U12, U13, U14, U15, U16` (and `U4, U8` only with the `hazmat` feature flag). This matches the NIST §5.2.1.2 set.

### 4.4 GHASH-Key Derivation

```rust
fn from(cipher: Aes) -> Self {
    let mut ghash_key = ghash::Key::default();   // 16 zero bytes
    cipher.encrypt_block(&mut ghash_key);        // H = E_K(0^128)
    let ghash = GHash::new(&ghash_key);
    #[cfg(feature = "zeroize")]
    ghash_key.zeroize();
    Self { cipher, ghash, nonce_size: PhantomData, tag_size: PhantomData }
}
```

This is **exactly** Algorithm 4 step 1: `H = CIPH_K(0^128)`. The H-key is zeroized after seeding GHash.

### 4.5 J0 Derivation (Counter Initialization)

```rust
fn init_ctr(&self, nonce: &Nonce<NonceSize>) -> (Ctr32BE<&Aes>, Block) {
    let j0 = if NonceSize::to_usize() == 12 {
        // 96-bit fast path: J0 = IV || 0^31 || 1
        let mut block = ghash::Block::default();
        block[..12].copy_from_slice(nonce);
        block[15] = 1;                            // 0^31 || 1 = 0x00 0x00 0x00 0x01
        block
    } else {
        // Variable-length IV: J0 = GHASH_H(IV || 0^{s+64} || [len(IV)]_64)
        let mut ghash = self.ghash.clone();
        ghash.update_padded(nonce);
        let mut block = ghash::Block::default();
        let nonce_bits = (NonceSize::to_usize() as u64) * 8;
        block[8..].copy_from_slice(&nonce_bits.to_be_bytes());
        ghash.update(&[block]);
        ghash.finalize()
    };
    let mut ctr = Ctr32BE::inner_iv_init(&self.cipher, &j0);
    let mut tag_mask = Block::default();
    ctr.write_keystream_block(&mut tag_mask);     // E_K(J0) cached for tag XOR
    (ctr, tag_mask)
}
```

Maps 1:1 to NIST Algorithm 4 step 2 + the counter-1 step 6 mask.

### 4.6 Encrypt / Decrypt

```rust
fn encrypt_inout_detached(&self, nonce, associated_data, mut buffer)
    -> Result<Tag<TagSize>, Error>
{
    if buffer.len() as u64 > P_MAX || associated_data.len() as u64 > A_MAX {
        return Err(Error);
    }
    let (ctr, mask) = self.init_ctr(nonce);
    ctr.apply_keystream_partial(buffer.reborrow());      // GCTR_K(inc_32(J0), P)
    let full_tag = self.compute_tag(mask, associated_data, buffer.get_out());
    Ok(Tag::try_from(&full_tag[..TagSize::to_usize()]).expect("tag size mismatch"))
}

fn decrypt_inout_detached(&self, nonce, associated_data, buffer, tag)
    -> Result<(), Error>
{
    if buffer.len() as u64 > P_MAX || associated_data.len() as u64 > A_MAX {
        return Err(Error);
    }
    let (ctr, mask) = self.init_ctr(nonce);
    let expected_tag = self.compute_tag(mask, associated_data, buffer.get_in());
    use subtle::ConstantTimeEq;
    if expected_tag[..TagSize::to_usize()].ct_eq(tag).into() {
        ctr.apply_keystream_partial(buffer);              // verify-then-decrypt
        Ok(())
    } else {
        Err(Error)
    }
}
```

Two correctness-critical patterns to copy:
- **Length check before any cipher work** — both `P_MAX` and `A_MAX` are enforced.
- **Verify-then-decrypt** in `decrypt_inout_detached` (tag is checked **before** the keystream is applied). This is the recommended order for batch APIs (avoids RUP exposure).
- **`subtle::ConstantTimeEq`** for tag comparison.

### 4.7 Tag Computation

```rust
fn compute_tag(&self, mask: Block, associated_data: &[u8], buffer: &[u8]) -> Tag {
    let mut ghash = self.ghash.clone();
    ghash.update_padded(associated_data);            // A || 0^v
    ghash.update_padded(buffer);                     // C || 0^u
    let associated_data_bits = (associated_data.len() as u64) * 8;
    let buffer_bits = (buffer.len() as u64) * 8;
    let mut block = ghash::Block::default();
    block[..8].copy_from_slice(&associated_data_bits.to_be_bytes());  // [len(A)]_64
    block[8..].copy_from_slice(&buffer_bits.to_be_bytes());           // [len(C)]_64
    ghash.update(&[block]);
    let mut tag = ghash.finalize();
    for (a, b) in tag.as_mut_slice().iter_mut().zip(mask.as_slice()) {
        *a ^= *b;                                                      // tag ^ E_K(J0)
    }
    tag
}
```

Exactly Algorithm 4 steps 4–6.

### 4.8 Where to Substitute Camellia (THE KEY DECISION)

The `aes-gcm` crate compiles **with no AES code** if the optional `aes` feature is disabled. Therefore:

```rust
// In libpna's Cargo.toml:
[dependencies]
aes-gcm = { version = "0.11.0-rc.3", default-features = false }
camellia = "0.2"

// In libpna source:
use aes_gcm::AesGcm;
use aes_gcm::aead::generic_array::typenum::U12;
use camellia::Camellia256;

pub type Camellia256Gcm = AesGcm<Camellia256, U12>;
```

That's it. `Camellia256` already implements `BlockSizeUser<BlockSize = U16> + BlockCipherEncrypt + KeyInit` (verified in `RustCrypto/block-ciphers/camellia/src/lib.rs` master branch — type aliases `Camellia128`, `Camellia192`, `Camellia256` defined; `BlockSize = U16` declared; `BlockCipherEncrypt` impl present). The trait surface is bit-identical to `Aes256`'s.

> **Important caveat**: The decision was made on 2026-04-24 to "DIY in-tree" Camellia-256-GCM. The above three-line composition is **not** DIY — it leverages the `aes-gcm` crate's generic GCM machinery. If "DIY" means hand-rolling GHASH, GCTR, and J0 from scratch (e.g. for a no-deps build, or for educational reasons, or to escape the `aes-gcm` API surface), the appropriate references are §1 (NIST pseudocode) plus the GHASH crate at <https://github.com/RustCrypto/universal-hashes/tree/master/ghash>. **The maintainer should clarify which "DIY" is intended before implementation begins.**

### 4.9 Useful Companion Crates

| Crate | Purpose | URL |
|---|---|---|
| `ghash` 0.6 | GF(2^128) universal hash | <https://github.com/RustCrypto/universal-hashes/tree/master/ghash> |
| `polyval` 0.7 | Underlying GF(2^128) field arithmetic with PCLMUL on x86 | <https://github.com/RustCrypto/universal-hashes/tree/master/polyval> |
| `ctr` 0.10 | Generic CTR mode (`Ctr32BE`, `Ctr64BE`, `Ctr128BE`) | <https://github.com/RustCrypto/block-modes> |
| `cipher` 0.5 | Trait surface (`BlockCipherEncrypt` etc) | <https://github.com/RustCrypto/traits/tree/master/cipher> |
| `subtle` 2.x | Constant-time comparison primitives | <https://github.com/dalek-cryptography/subtle> |
| `aead` 0.6 | High-level AEAD trait surface | <https://github.com/RustCrypto/traits/tree/master/aead> |

---

## 5. GCM Implementation Pitfalls (Bug & CVE Catalog)

### 5.1 Joux 2006 — The Foundational Nonce-Reuse Attack

Source: A. Joux, "Authentication Failures in NIST version of GCM", 2006, archived at NIST CSRC: <https://csrc.nist.gov/groups/ST/toolkit/BCM/documents/comments/800-38_Series-Drafts/GCM/Joux_comments.pdf>. Cited as Reference [5] in NIST SP 800-38D Appendix E.

**Attack mechanism**: When the same `(K, IV)` pair is used twice, the attacker collects two ciphertext/tag pairs `(C, T)` and `(C', T')` for the same nonce. Subtracting their tag equations yields a polynomial in H of known coefficients. Solving for the roots reveals at most ~m candidate values for H. With a couple of additional collisions the attacker uniquely recovers H. **Once H is known, the attacker can forge tags on any `(C, A)` pair under that key.**

NIST SP 800-38D §8 calls this out directly:

> "if IVs are ever repeated for the GCM authenticated encryption function for a given key, then it is likely that an adversary will be able to determine the hash subkey from the resulting ciphertexts. The adversary then could easily construct a ciphertext forgery." [App. A, p25]

### 5.2 Bock, Zauner, Devine 2016 — Real-World Nonce Reuse in HTTPS

Source: H. Böck, A. Zauner, S. Devlin, J. Somorovsky, P. Jovanovic, "Nonce-Disrespecting Adversaries: Practical Forgery Attacks on GCM in TLS", USENIX WOOT'16. <https://www.usenix.org/conference/woot16/workshop-program/presentation/bock>; eprint <https://eprint.iacr.org/2016/475>; PoC <https://github.com/nonce-disrespect/nonce-disrespect>.

**Findings**:
- Internet-wide scan identified **184 HTTPS servers** that repeated GCM nonces, fully breaking authenticity.
- **>70,000 servers** used random nonces, putting them at birthday-bound risk if enough data was sent under one session.
- Affected servers included financial institutions and a credit card company.

**Lesson for PNA**: The chunk-encryption AAD scheme **must** make IV reuse impossible by construction — derive per-entry/per-chunk IVs from a key-committed counter, not from a freshly-sampled RNG, and never reuse a key between archives without HKDF subkey derivation.

### 5.3 GHASH Endianness / Bit-Reflection Bugs

The bit-reflection convention in GCM (§1.5 above) is the **single most common** GHASH bug source historically. Symptoms:
- Tags match for short messages (≤16 bytes) but diverge for multi-block messages.
- Tags match all-zero AAD/PT but diverge once any non-zero data is added.
- Test vectors from NIST CAVP fail at random.

Mitigation:
- Use `polyval`/`ghash` from RustCrypto rather than rolling your own GF(2^128).
- Test against AES-GCM Test Case 13 (§3.4 above) **first** — wrong endianness typically gives an off-by-everything tag.

### 5.4 Tag Comparison Timing-Attack

Comparing the computed and received tags with `==` (or `memcmp`) leaks timing information that lets an attacker forge tags one byte at a time.

**Required pattern** (matches `aes-gcm` source, §4.6):
```rust
use subtle::ConstantTimeEq;
if expected_tag.ct_eq(received_tag).into() { ... } else { return Err(...); }
```

Ban `assert_eq!(tag1, tag2)` and `tag1 == tag2` in any decryption path. Add a CI lint or grep-based pre-commit check.

### 5.5 Counter Wraparound (`inc_32` Overflow)

After 2^32 `inc_32` increments, the counter wraps and reuses the post-J0 keystream. For 16-byte blocks this corresponds to 2^32 × 16 = 64 GiB of ciphertext under a single `(K, IV)`.

NIST P_MAX (`2^39 − 256` bits = 64 GiB − 32 bytes) is **explicitly designed to prevent this wrap**. Enforcing `P_MAX` in the encryption path makes the wrap impossible.

PNA gotcha: A multipart archive with a single per-archive key and large file content could approach P_MAX. The §6 AAD design must include an entry/chunk index so each independent encryption stays well below P_MAX. **Recommended hard limit per (subkey, IV) invocation: 2^30 bytes (1 GiB)** — comfortably below NIST's 64 GiB and leaving safety margin for the n/2^t targeted-forgery attack.

### 5.6 RUP — Releasing Unverified Plaintext

In streaming decryption, an implementation may emit decrypted bytes **before** verifying the tag (because the tag is only available after the last block). If the consumer of those bytes acts on them irreversibly (writes to a file, sends to network, runs as code), the tag verification at the end can no longer undo the damage.

GCM is **not** RUP-secure as a primitive (in the formal sense; see Andreeva et al. 2014, <https://eprint.iacr.org/2014/144.pdf>).

Mitigations for PNA streaming decrypt:
- **Buffer-then-verify**: For files smaller than a threshold (say 64 MiB), buffer the entire ciphertext and verify the tag before yielding any plaintext byte.
- **Chunked AEAD**: For larger files, slice into independent chunks (e.g. 64 KiB à la age STREAM, §6.2 below). Each chunk has its own tag; tag failure on chunk i discards chunk i but does not retroactively undo chunks 0..i-1. This is the **only** safe streaming pattern.
- **Never** stream raw GCM plaintext from a single huge `(K, IV)` to an irreversible sink.

### 5.7 The "All-Zero H" Edge Case

If `H = E_K(0^128)` happens to equal zero (probability 2^-128, so essentially impossible for AES, **but worth documenting**), GHASH degenerates and produces tag = `E_K(J0)` regardless of input. **Action**: do not add a check (it would be theater); rely on the security argument.

### 5.8 Pitfalls Specific to Camellia

Camellia-256 uses 24 rounds (vs Camellia-128's 18). The RustCrypto `Camellia256` type alias (`Camellia<U32, 34>`) declares 34 round-keys, matching the F-function requirements. We trust RustCrypto's implementation as test-vector-validated against CRYPTREC ECB vectors (`src/tests/data/block/camellia.vec`).

---

## 6. AAD Design Best Practices

### 6.1 TLS 1.3 Record Layer (RFC 8446)

Source: RFC 8446 §5.2, §5.3, §5.5. <https://datatracker.ietf.org/doc/html/rfc8446>

- **Per-record nonce**: `nonce = static_iv XOR sequence_number_padded` [RFC 8446 §5.3]. Sequence number is a 64-bit big-endian counter, left-padded to nonce length with zeros. Static IV is per-direction, derived via HKDF.
- **AAD**: `record_type || record_version || record_length` (5 bytes) [RFC 8446 §5.2].
- **Record size cap**: ciphertext ≤ 2^14 + 256 bytes [RFC 8446 §5.2 / §5.4]. The cap is **deliberately small** so per-record AEAD limits never bite.
- **Key usage limit**: TLS 1.3 mandates rekey after a per-AEAD limit (`KeyUpdate` message).

### 6.2 age STREAM (C2SP age v1)

Source: <https://github.com/C2SP/C2SP/blob/main/age.md>

- **Chunk size**: 64 KiB.
- **AEAD**: ChaCha20-Poly1305.
- **Per-chunk nonce**: 12 bytes = 11-byte big-endian chunk counter (starts at 0) || 1-byte last-chunk flag (0x00 = not last, 0x01 = last).
- **AAD**: empty.

The trick: encoding the "last chunk" bit *into the nonce itself* makes truncation attacks impossible — flipping the last-chunk bit changes the nonce, which reroutes the keystream and breaks decryption of that chunk.

**For PNA**: this is the **gold-standard streaming AEAD pattern**.

### 6.3 libsodium secretstream

Source: <https://doc.libsodium.org/secret-key_cryptography/secretstream>

- **AEAD**: ChaCha20-Poly1305 (via XChaCha20-Poly1305 stream construction).
- **Per-chunk nonce**: counter `i` (32-bit) || derived value `n` (64-bit). After each encryption, **the nonce is XORed with the first 8 bytes of the auth tag** (forward-secrecy / confounding step).
- **AAD**: optional, application-supplied.
- **Tag types**: `MESSAGE`, `PUSH`, `REKEY`, `FINAL` — encoded in the message itself, not the nonce.

### 6.4 What to Put in PNA's AAD

Suggested minimum (which the current spec draft already implements):
```
AAD = format_magic (e.g. b"PNA-v1-AEAD") ||
      encryption_byte ||
      mode_byte ||
      archive_nonce_salt (8 bytes) ||
      archive_kdf_salt (8 bytes) ||
      entry_salt (8 bytes) ||
      entry_index_be32 ||
      chunk_index_be32 ||
      is_final_chunk (1 byte) ||
      metadata_hash (32 bytes SHA-256)
```

This binds every ciphertext block to:
- a specific format version (prevents cross-version downgrade)
- a specific archive identity (prevents cross-archive cut-and-paste)
- a specific entry position (prevents reordering within archive)
- a specific chunk position (prevents reordering within entry)
- the chunk's role as last/non-last (prevents truncation)
- entry metadata (prevents Kohno-style filename/permission tampering)

### 6.5 What NOT to Put in AAD

- **Do not put the key or any subkey in AAD.** AAD is authenticated, not confidential.
- **Do not put the nonce in AAD.** Nonce is already authenticated implicitly (it's the GCTR seed).
- **Do not put user-supplied filenames in AAD as raw UTF-8.** Normalize first; otherwise a different normalization in the decoder produces a bogus FAIL.
- **Do not put timestamps in AAD** unless you genuinely require time-binding.

---

## 7. 96-bit Nonce vs Random Nonce vs Counter Nonce

### 7.1 NIST Recommendation

> "For IVs, it is recommended that implementations restrict support to the length of 96 bits, to promote interoperability, efficiency, and simplicity of design." [NIST SP 800-38D §5.2.1.1, p8]

96 bits is the only IV length that benefits from the GCM "fast path" (`J_0 = IV || 0^31 || 1` instead of a full GHASH pre-computation), and it is the only length supported by RFC 5116's AEAD interface (`N_MIN = N_MAX = 12`).

**Rule for PNA**: always use 96-bit nonces.

### 7.2 The 2^32 Invocation Cap (and how to escape it)

Per §1.10 above:

> "The total number of invocations of the authenticated encryption function shall not exceed 2^32 ... unless an implementation only uses 96-bit IVs that are generated by the deterministic construction" [NIST SP 800-38D §8.3, p21]

This means:
- **Random 96-bit nonce + same key**: birthday bound at 2^48 invocations gives 2^-32 collision probability, hence the 2^32 cap.
- **Deterministic 96-bit nonce (fixed-field || invocation-counter) + same key**: cap is `2^s` where s = invocation counter bit width. With s=64, cap is 2^64 (effectively unlimited).
- **Per-entry HKDF subkey + counter-0 nonce**: each entry has a fresh key, so each entry's invocation count is independent. PNA can have arbitrarily many entries per archive without hitting any cap.

### 7.3 HKDF Subkey Derivation Resets the Cap

Subkeys derived via HKDF (RFC 5869) are independent for security analysis purposes — each subkey starts fresh with its own 2^32 (random nonce) or 2^64 (counter nonce) budget. **This is the standard practice and is implicit in TLS 1.3's KeyUpdate, age's per-recipient subkeys, and Noise Protocol's chain-key advancement.**

**Recommended PNA pattern** (already in spec draft):

```text
master_key  -->  HKDF-Extract(salt = archive_random_salt, IKM = master_key) = PRK
                 HKDF-Expand(PRK, info = b"PNA-v1-AEAD-..." || algorithm_name, L = 32) = entry_key
                 use entry_key with counter-style 96-bit nonce per chunk
```

With this design:
- Each entry has its own `entry_key`. Per-entry invocation cap is `2^64` (deterministic counter), so a single entry can have up to 2^64 chunks (effectively unlimited).
- Different entries use different keys, so cross-entry IV-reuse is mathematically impossible.
- Compromise of one `entry_key` does not affect other entries (forward / backward isolation per entry).

### 7.4 Why Random Nonces are Problematic

Random 96-bit nonces have a 2^-32 collision probability after 2^32 messages (birthday bound). For an archive with millions of files this is fine in expectation but **a single-bit fault in the RNG can cause silent reuse**. Bock 2016 shows real-world examples of exactly this failure mode in TLS implementations.

**Don't use random nonces in PNA.** Use deterministic counter-style nonces with per-entry HKDF subkey derivation as in §7.3.

---

## 8. Quick-Reference Summary for the Implementer

### 8.1 The Camellia-256-GCM Algorithm at a Glance

For each entry/chunk:
1. Derive `entry_key = HKDF-Expand(archive_PRK, "PNA-v1-AEAD-Camellia-256-GCM" || entry_index, 32)` (32 bytes for Camellia-256).
2. Construct nonce: `nonce = nonce_salt(8B) || chunk_index_be4` (12 bytes total, deterministic).
3. Construct AAD as in §6.4.
4. `(C, T) = Camellia256-GCM_encrypt(entry_key, nonce, AAD, plaintext)` with `t = 128`.
5. Store `C || T` (nonce is reconstructible from chunk position).

### 8.2 Algorithm Verification Checklist

- [ ] H derivation: `H = Camellia256_encrypt(K, 0^128)` — sanity-check against vector (§3.4 pattern with Camellia substituted).
- [ ] J0 (96-bit IV): `J0 = IV || 0x00 0x00 0x00 0x01` — verify last byte is 1.
- [ ] Counter starts at J0+1 for plaintext encryption.
- [ ] GHASH input order: `A || 0^v || C || 0^u || [len(A)]_64 || [len(C)]_64`. **Bit lengths, not byte lengths**, in the trailing block.
- [ ] Tag = `MSB_t(GCTR_K(J0, S))` = `MSB_t(E_K(J0) ^ S)` (since GCTR on a single block is just XOR).
- [ ] Length checks before any cipher work: `len(P) <= P_MAX`, `len(A) <= A_MAX`.
- [ ] Tag comparison via `subtle::ConstantTimeEq`.

### 8.3 Test-Vector Generation Plan

1. **Phase 1**: Implement against AES-256-GCM first. Verify with NIST CAVP vectors.
2. **Phase 2**: Swap `Aes256` → `Camellia256` (single-line type alias change per §4.8 if using composition approach).
3. **Phase 3**: Generate 10+ Camellia-256-GCM vectors from Botan AND Crypto++ independently. Cross-check bit-exact equality. Commit as `lib/tests/test-vectors/camellia_256_gcm.json`.
4. **Phase 4**: Add fuzz harness (`fuzz/camellia_gcm`) using same harness pattern as existing `aes_ctr` / `camellia_ctr` targets. Fuzz the round-trip property: `decrypt(encrypt(x)) == x` for arbitrary `(K, IV, A, P)`.

### 8.4 Risks and Open Questions

1. **DIY scope ambiguity** (§4.8 / §0): Is "DIY" three-line generic composition, or hand-rolled GHASH/GCTR? **Decision needed before implementation begins.**
2. **Camellia-GCM has no canonical test vectors.** PNA may end up being a *de facto* reference for the construction. The cross-implementation verification step (§8.3 phase 3) is mandatory.
3. **Performance.** Camellia lacks AES-NI-equivalent silicon support on x86-64 (until very recent AVX-512 GFNI variants exist in Botan but not RustCrypto). Expect Camellia-256-GCM to be 5-10× slower than AES-256-GCM on most hardware. **未確認 exact ratio for RustCrypto's `camellia` 0.2 crate.**
4. **Standards compliance gap.** RFC 6367 names the construction but does not formally publish it as an IETF AEAD. Some compliance audits (e.g. FIPS 140-3) may not approve Camellia-GCM. PNA users should be warned in docs that AES-256-GCM is the FIPS-approved choice; Camellia-256-GCM is provided for jurisdictional / algorithmic-diversity reasons (CRYPTREC, ISO/IEC 18033-3).

---

## 9. Source Index (One-Click Reference)

Primary specs:
- NIST SP 800-38D: <https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf>
- RFC 6367 (Camellia in TLS): <https://datatracker.ietf.org/doc/html/rfc6367>
- RFC 5116 (AEAD interface): <https://datatracker.ietf.org/doc/html/rfc5116>
- RFC 5288 (AES-GCM in TLS): <https://datatracker.ietf.org/doc/html/rfc5288>
- RFC 8446 (TLS 1.3): <https://datatracker.ietf.org/doc/html/rfc8446>
- RFC 5869 (HKDF): <https://datatracker.ietf.org/doc/html/rfc5869>
- RFC 3713 (Camellia primitive): <https://datatracker.ietf.org/doc/html/rfc3713>

Reference implementations:
- RustCrypto AEADs: <https://github.com/RustCrypto/AEADs>
- RustCrypto AEADs `aes-gcm` lib.rs: <https://github.com/RustCrypto/AEADs/blob/master/aes-gcm/src/lib.rs>
- RustCrypto block-ciphers `camellia`: <https://github.com/RustCrypto/block-ciphers/tree/master/camellia>
- RustCrypto universal-hashes `ghash`: <https://github.com/RustCrypto/universal-hashes/tree/master/ghash>
- Botan Camellia: <https://github.com/randombit/botan/tree/master/src/lib/block/camellia>
- Botan AEAD/GCM tests: <https://github.com/randombit/botan/blob/master/src/tests/data/aead/gcm.vec>
- BouncyCastle Camellia: <https://github.com/bcgit/bc-java/blob/main/core/src/main/java/org/bouncycastle/crypto/engines/CamelliaEngine.java>

Security literature:
- McGrew & Viega 2004 (GCM security): <https://eprint.iacr.org/2004/193>
- Joux 2006 (NIST GCM nonce reuse): <https://csrc.nist.gov/groups/ST/toolkit/BCM/documents/comments/800-38_Series-Drafts/GCM/Joux_comments.pdf>
- Böck/Zauner/Devine 2016 (USENIX): <https://eprint.iacr.org/2016/475>
- Andreeva et al. 2014 (RUP formalization): <https://eprint.iacr.org/2014/144.pdf>

Streaming AEAD designs to learn from:
- age v1: <https://github.com/C2SP/C2SP/blob/main/age.md>
- libsodium secretstream: <https://doc.libsodium.org/secret-key_cryptography/secretstream>

CRYPTREC / Camellia primitive references:
- NTT Camellia portal: <https://info.isl.ntt.co.jp/crypt/eng/camellia/>

---

End of reference document.
