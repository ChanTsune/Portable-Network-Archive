//! STREAM-based GCM segment encryption writer.

use crate::cipher::aead::{StreamHeader, segment_nonce};
use aes_gcm::AesGcm;
use aes_gcm::aead::array::Array;
use aes_gcm::aead::{Aead, AeadCore, KeyInit, consts::U12};
use std::io::{self, Write};

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
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cipher::aead::GCM_TAG_LEN;
    use aes::Aes256;
    use camellia::Camellia256;

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
}
