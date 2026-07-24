//! STREAM-based GCM segment encryption writer and decryption reader.

use crate::cipher::aead::{GCM_TAG_LEN, StreamHeader, segment_nonce};
use crate::error::AeadError;
use aes_gcm::AesGcm;
use aes_gcm::aead::array::Array;
use aes_gcm::aead::{Aead, AeadCore, KeyInit, consts::U12};
use std::io::{self, Read, Write};

pub(crate) struct GcmEncryptWriter<W, C>
where
    AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
{
    w: W,
    cipher: AesGcm<C, U12>,
    nonce_prefix: [u8; 7],
    segment_size: usize,
    counter: u32,
    buf: Vec<u8>,
}

impl<W, C> GcmEncryptWriter<W, C>
where
    W: Write,
    AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
{
    pub(crate) fn new(writer: W, k_stream: &[u8; 32], header: &StreamHeader) -> Self {
        let segment_size = header.segment_size() as usize;
        Self {
            w: writer,
            cipher: AesGcm::<C, U12>::new_from_slice(k_stream)
                .expect("32-byte stream key length matches cipher key size"),
            nonce_prefix: header.nonce_prefix,
            segment_size,
            counter: 0,
            buf: Vec::with_capacity(segment_size),
        }
    }

    fn flush_segment(&mut self, is_final: bool) -> io::Result<()> {
        let nonce =
            Array::<u8, U12>::from(segment_nonce(&self.nonce_prefix, self.counter, is_final));
        let segment = self
            .cipher
            .encrypt(&nonce, self.buf.as_slice())
            .map_err(|_| io::Error::other("GCM segment encryption failed"))?;
        self.w.write_all(&segment)?;
        self.buf.clear();
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or_else(|| io::Error::other("GCM segment counter overflow"))?;
        Ok(())
    }

    pub(crate) fn finish(mut self) -> io::Result<W> {
        self.flush_segment(true)?;
        Ok(self.w)
    }

    #[inline]
    pub(crate) fn get_mut(&mut self) -> &mut W {
        &mut self.w
    }
}

impl<W, C> Write for GcmEncryptWriter<W, C>
where
    W: Write,
    AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut rest = buf;
        while !rest.is_empty() {
            let space = self.segment_size - self.buf.len();
            let take = space.min(rest.len());
            self.buf.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.buf.len() == self.segment_size && !rest.is_empty() {
                self.flush_segment(false)?;
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

#[derive(Copy, Clone)]
enum Fuse {
    Malformed(&'static str),
    Authentication,
    Truncation,
}

impl From<Fuse> for AeadError {
    #[inline]
    fn from(f: Fuse) -> Self {
        match f {
            Fuse::Malformed(detail) => AeadError::Malformed(detail),
            Fuse::Authentication => AeadError::AuthenticationFailure,
            Fuse::Truncation => AeadError::Truncation,
        }
    }
}

/// STREAM-based GCM segment decryption reader.
///
/// Reads one segment ahead so that a segment is decrypted with the final flag
/// set only when no datastream bytes follow it. Only tag-verified plaintext is
/// ever returned to the caller.
pub(crate) struct GcmDecryptReader<R, C>
where
    AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
{
    r: R,
    cipher: AesGcm<C, U12>,
    nonce_prefix: [u8; 7],
    segment_size: usize,
    counter: u32,
    pending: Option<Vec<u8>>,
    started: bool,
    plain: Vec<u8>,
    pos: usize,
    done: bool,
    fuse: Option<Fuse>,
}

impl<R, C> GcmDecryptReader<R, C>
where
    R: Read,
    AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
{
    pub(crate) fn new(reader: R, k_stream: &[u8; 32], header: &StreamHeader) -> Self {
        Self {
            r: reader,
            cipher: AesGcm::<C, U12>::new_from_slice(k_stream)
                .expect("32-byte stream key length matches cipher key size"),
            nonce_prefix: header.nonce_prefix,
            segment_size: header.segment_size() as usize,
            counter: 0,
            pending: None,
            started: false,
            plain: Vec::new(),
            pos: 0,
            done: false,
            fuse: None,
        }
    }

    fn fail(&mut self, f: Fuse) -> io::Error {
        self.fuse = Some(f);
        AeadError::from(f).into()
    }

    fn read_full(&mut self) -> io::Result<Vec<u8>> {
        // The segment size comes from the unauthenticated stream header, so a
        // hostile archive controls this allocation; fail with an error instead
        // of aborting on exhaustion.
        let cap = self.segment_size + GCM_TAG_LEN;
        let mut buf = Vec::new();
        buf.try_reserve_exact(cap).map_err(|_| {
            io::Error::new(
                io::ErrorKind::OutOfMemory,
                format!("failed to allocate {cap} bytes for segment"),
            )
        })?;
        buf.resize(cap, 0);
        let mut filled = 0;
        while filled < cap {
            match self.r.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        buf.truncate(filled);
        Ok(buf)
    }

    fn refill(&mut self) -> io::Result<()> {
        if !self.started {
            self.pending = Some(self.read_full()?);
            self.started = true;
        }
        let b = self.read_full()?;
        let a = self
            .pending
            .take()
            .expect("a started reader always holds a buffered segment");
        let i = self.counter;
        if b.is_empty() {
            if a.len() < GCM_TAG_LEN {
                // Zero segments is a structural violation (no empty final segment
                // was even present); a short tail after earlier segments is a cut.
                let f = if i == 0 {
                    Fuse::Malformed("datastream shorter than one empty final segment")
                } else {
                    Fuse::Truncation
                };
                return Err(self.fail(f));
            }
            let plain = self.decrypt(i, true, &a)?;
            self.plain = plain;
            self.pos = 0;
            self.done = true;
        } else {
            if a.len() < self.segment_size + GCM_TAG_LEN {
                return Err(self.fail(Fuse::Malformed(
                    "non-final segment shorter than segment size",
                )));
            }
            let plain = self.decrypt(i, false, &a)?;
            self.counter = self
                .counter
                .checked_add(1)
                .ok_or_else(|| self.fail(Fuse::Malformed("segment counter overflow")))?;
            self.plain = plain;
            self.pos = 0;
            self.pending = Some(b);
        }
        Ok(())
    }

    fn decrypt(&mut self, counter: u32, is_final: bool, segment: &[u8]) -> io::Result<Vec<u8>> {
        let nonce = Array::<u8, U12>::from(segment_nonce(&self.nonce_prefix, counter, is_final));
        match self.cipher.decrypt(&nonce, segment) {
            Ok(plain) => Ok(plain),
            Err(_) => Err(self.fail(Fuse::Authentication)),
        }
    }
}

impl<R, C> Read for GcmDecryptReader<R, C>
where
    R: Read,
    AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(f) = self.fuse {
            return Err(AeadError::from(f).into());
        }
        if self.pos >= self.plain.len() {
            if self.done {
                return Ok(0);
            }
            self.refill()?;
            if self.pos >= self.plain.len() {
                return Ok(0);
            }
        }
        let n = (self.plain.len() - self.pos).min(buf.len());
        buf[..n].copy_from_slice(&self.plain[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AeadError;
    use aes::Aes256;
    use camellia::Camellia256;
    use std::io::{Cursor, Read};

    const KEY: [u8; 32] = [7u8; 32];
    const PREFIX: [u8; 7] = [3u8; 7];
    const SEG: u32 = 4;

    fn header(segment_size: u32) -> StreamHeader {
        StreamHeader::new([0u8; 32], PREFIX, segment_size).unwrap()
    }

    fn encrypt_all<C>(plain: &[u8]) -> Vec<u8>
    where
        AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
    {
        let mut w = GcmEncryptWriter::<_, C>::new(Vec::new(), &KEY, &header(SEG));
        w.write_all(plain).unwrap();
        w.finish().unwrap()
    }

    fn encrypt_byte_by_byte<C>(plain: &[u8]) -> Vec<u8>
    where
        AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
    {
        let mut w = GcmEncryptWriter::<_, C>::new(Vec::new(), &KEY, &header(SEG));
        for b in plain {
            w.write_all(std::slice::from_ref(b)).unwrap();
        }
        w.finish().unwrap()
    }

    fn decrypt_stream<C>(plain_len: usize, ciphertext: &[u8]) -> Vec<u8>
    where
        AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
    {
        let cipher = AesGcm::<C, U12>::new_from_slice(&KEY).unwrap();
        let non_final = if plain_len == 0 {
            0
        } else {
            (plain_len - 1) / SEG as usize
        };
        let mut out = Vec::new();
        let mut rest = ciphertext;
        let mut counter = 0u32;
        for _ in 0..non_final {
            let nonce = Array::<u8, U12>::from(segment_nonce(&PREFIX, counter, false));
            let (segment, tail) = rest.split_at(SEG as usize + GCM_TAG_LEN);
            out.extend_from_slice(&cipher.decrypt(&nonce, segment).unwrap());
            rest = tail;
            counter += 1;
        }
        let nonce = Array::<u8, U12>::from(segment_nonce(&PREFIX, counter, true));
        out.extend_from_slice(&cipher.decrypt(&nonce, rest).unwrap());
        out
    }

    #[test]
    fn empty_plaintext_emits_single_tag_only_segment() {
        let ct = encrypt_all::<Aes256>(b"");
        assert_eq!(ct.len(), GCM_TAG_LEN);
        assert_eq!(decrypt_stream::<Aes256>(0, &ct).as_slice(), b"");
    }

    #[test]
    fn below_segment_size_emits_single_final_segment() {
        let plain = b"abc";
        let ct = encrypt_all::<Aes256>(plain);
        assert_eq!(ct.len(), plain.len() + GCM_TAG_LEN);
        assert_eq!(decrypt_stream::<Aes256>(plain.len(), &ct).as_slice(), plain);
    }

    #[test]
    fn exact_segment_size_has_no_trailing_empty_segment() {
        let plain = b"abcd";
        let ct = encrypt_all::<Aes256>(plain);
        assert_eq!(ct.len(), plain.len() + GCM_TAG_LEN);
        assert_eq!(decrypt_stream::<Aes256>(plain.len(), &ct).as_slice(), plain);
    }

    #[test]
    fn two_segments_split_into_non_final_and_final() {
        let plain = b"abcdefgh";
        let ct = encrypt_all::<Aes256>(plain);
        assert_eq!(ct.len(), plain.len() + 2 * GCM_TAG_LEN);
        assert_eq!(decrypt_stream::<Aes256>(plain.len(), &ct).as_slice(), plain);
    }

    #[test]
    fn partial_tail_after_two_full_segments() {
        let plain = b"abcdefghi";
        let ct = encrypt_all::<Aes256>(plain);
        assert_eq!(ct.len(), plain.len() + 3 * GCM_TAG_LEN);
        assert_eq!(decrypt_stream::<Aes256>(plain.len(), &ct).as_slice(), plain);
    }

    #[test]
    fn output_independent_of_write_boundaries() {
        let plain = b"abcdefghi";
        assert_eq!(
            encrypt_all::<Aes256>(plain),
            encrypt_byte_by_byte::<Aes256>(plain)
        );
    }

    #[test]
    fn camellia_roundtrip() {
        let plain = b"abcdefgh";
        let ct = encrypt_all::<Camellia256>(plain);
        assert_eq!(
            decrypt_stream::<Camellia256>(plain.len(), &ct).as_slice(),
            plain
        );
    }

    fn decrypt_all<C>(ciphertext: Vec<u8>) -> io::Result<Vec<u8>>
    where
        AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
    {
        let mut r = GcmDecryptReader::<_, C>::new(Cursor::new(ciphertext), &KEY, &header(SEG));
        let mut out = Vec::new();
        r.read_to_end(&mut out)?;
        Ok(out)
    }

    fn roundtrip<C>(plain: &[u8])
    where
        AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
    {
        let ct = encrypt_all::<C>(plain);
        assert_eq!(decrypt_all::<C>(ct).unwrap().as_slice(), plain);
    }

    fn classify(err: &io::Error) -> &AeadError {
        err.get_ref()
            .and_then(|e| e.downcast_ref::<AeadError>())
            .expect("decrypt error carries an AeadError")
    }

    struct OneByteReader<R>(R);

    impl<R: Read> Read for OneByteReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if buf.is_empty() {
                return Ok(0);
            }
            self.0.read(&mut buf[..1])
        }
    }

    struct InterruptingReader<R> {
        inner: R,
        armed: bool,
    }

    impl<R: Read> Read for InterruptingReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.armed {
                self.armed = false;
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            self.armed = true;
            self.inner.read(buf)
        }
    }

    struct StallReader {
        segments: Vec<Vec<u8>>,
        index: usize,
        pos: usize,
    }

    impl Read for StallReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.index >= self.segments.len() || buf.is_empty() {
                return Ok(0);
            }
            let current = &self.segments[self.index];
            if self.pos >= current.len() {
                self.index += 1;
                self.pos = 0;
                return Ok(0);
            }
            let n = (current.len() - self.pos).min(buf.len());
            buf[..n].copy_from_slice(&current[self.pos..self.pos + n]);
            self.pos += n;
            Ok(n)
        }
    }

    #[test]
    fn roundtrip_empty_aes() {
        roundtrip::<Aes256>(b"");
    }

    #[test]
    fn roundtrip_three_bytes_aes() {
        roundtrip::<Aes256>(b"abc");
    }

    #[test]
    fn roundtrip_exact_segment_aes() {
        roundtrip::<Aes256>(b"abcd");
    }

    #[test]
    fn roundtrip_two_segments_aes() {
        roundtrip::<Aes256>(b"abcdefgh");
    }

    #[test]
    fn roundtrip_partial_tail_aes() {
        roundtrip::<Aes256>(b"abcdefghi");
    }

    #[test]
    fn roundtrip_empty_camellia() {
        roundtrip::<Camellia256>(b"");
    }

    #[test]
    fn roundtrip_partial_tail_camellia() {
        roundtrip::<Camellia256>(b"abcdefghi");
    }

    #[test]
    fn roundtrip_survives_one_byte_at_a_time() {
        let plain = b"abcdefghi";
        let ct = encrypt_all::<Aes256>(plain);
        let mut r =
            GcmDecryptReader::<_, Aes256>::new(OneByteReader(Cursor::new(ct)), &KEY, &header(SEG));
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out.as_slice(), plain);
    }

    #[test]
    fn roundtrip_survives_interrupted_reads() {
        let plain = b"abcdefghi";
        let ct = encrypt_all::<Aes256>(plain);
        let mut r = GcmDecryptReader::<_, Aes256>::new(
            InterruptingReader {
                inner: Cursor::new(ct),
                armed: true,
            },
            &KEY,
            &header(SEG),
        );
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out.as_slice(), plain);
    }

    #[test]
    fn flipped_ciphertext_byte_is_authentication_failure() {
        let mut ct = encrypt_all::<Aes256>(b"abcdefgh");
        ct[0] ^= 0x01;
        let err = decrypt_all::<Aes256>(ct).unwrap_err();
        assert!(matches!(classify(&err), AeadError::AuthenticationFailure));
    }

    #[test]
    fn flipped_tag_byte_is_authentication_failure() {
        let mut ct = encrypt_all::<Aes256>(b"abc");
        let last = ct.len() - 1;
        ct[last] ^= 0x01;
        let err = decrypt_all::<Aes256>(ct).unwrap_err();
        assert!(matches!(classify(&err), AeadError::AuthenticationFailure));
    }

    #[test]
    fn short_non_final_segment_with_trailing_bytes_is_malformed() {
        let reader = StallReader {
            segments: vec![vec![0u8; SEG as usize + GCM_TAG_LEN - 1], vec![0u8; 1]],
            index: 0,
            pos: 0,
        };
        let mut r = GcmDecryptReader::<_, Aes256>::new(reader, &KEY, &header(SEG));
        let mut out = [0u8; 8];
        let err = r.read(&mut out).unwrap_err();
        assert!(matches!(classify(&err), AeadError::Malformed(_)));
    }

    #[test]
    fn fifteen_byte_stream_is_malformed() {
        let err = decrypt_all::<Aes256>(vec![0u8; GCM_TAG_LEN - 1]).unwrap_err();
        assert!(matches!(classify(&err), AeadError::Malformed(_)));
    }

    #[test]
    fn full_segment_then_short_final_is_truncation() {
        let ct = encrypt_all::<Aes256>(b"abcdefgh");
        let mut truncated = ct[..SEG as usize + GCM_TAG_LEN].to_vec();
        truncated.extend_from_slice(&ct[SEG as usize + GCM_TAG_LEN..][..GCM_TAG_LEN - 1]);
        let err = decrypt_all::<Aes256>(truncated).unwrap_err();
        assert!(matches!(classify(&err), AeadError::Truncation));
    }

    #[test]
    fn swapped_segments_are_authentication_failure() {
        let ct = encrypt_all::<Aes256>(b"abcdefgh");
        let seg = SEG as usize + GCM_TAG_LEN;
        let mut swapped = ct[seg..].to_vec();
        swapped.extend_from_slice(&ct[..seg]);
        let err = decrypt_all::<Aes256>(swapped).unwrap_err();
        assert!(matches!(classify(&err), AeadError::AuthenticationFailure));
    }

    #[test]
    fn duplicated_segment_is_authentication_failure() {
        let ct = encrypt_all::<Aes256>(b"abcdefgh");
        let seg = SEG as usize + GCM_TAG_LEN;
        let mut duplicated = ct[..seg].to_vec();
        duplicated.extend_from_slice(&ct);
        let err = decrypt_all::<Aes256>(duplicated).unwrap_err();
        assert!(matches!(classify(&err), AeadError::AuthenticationFailure));
    }

    #[test]
    fn removed_final_segment_is_authentication_failure() {
        let ct = encrypt_all::<Aes256>(b"abcdefgh");
        let seg = SEG as usize + GCM_TAG_LEN;
        let err = decrypt_all::<Aes256>(ct[..seg].to_vec()).unwrap_err();
        assert!(matches!(classify(&err), AeadError::AuthenticationFailure));
    }

    #[test]
    fn error_is_reproduced_on_subsequent_reads() {
        let mut ct = encrypt_all::<Aes256>(b"abcdefgh");
        ct[0] ^= 0x01;
        let mut r = GcmDecryptReader::<_, Aes256>::new(Cursor::new(ct), &KEY, &header(SEG));
        let mut out = [0u8; 8];
        let first = r.read(&mut out).unwrap_err();
        let second = r.read(&mut out).unwrap_err();
        assert!(matches!(classify(&first), AeadError::AuthenticationFailure));
        assert!(matches!(
            classify(&second),
            AeadError::AuthenticationFailure
        ));
    }

    #[test]
    fn verified_plaintext_precedes_a_later_error() {
        let mut ct = encrypt_all::<Aes256>(b"abcdefgh");
        let tag_start = 2 * (SEG as usize) + GCM_TAG_LEN;
        ct[tag_start] ^= 0x01;
        let mut r = GcmDecryptReader::<_, Aes256>::new(Cursor::new(ct), &KEY, &header(SEG));
        let mut first = [0u8; 4];
        assert_eq!(r.read(&mut first).unwrap(), 4);
        assert_eq!(&first, b"abcd");
        let err = r.read(&mut first).unwrap_err();
        assert!(matches!(classify(&err), AeadError::AuthenticationFailure));
    }
}
