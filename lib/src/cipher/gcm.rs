//! STREAM-based GCM segment encryption writer and decryption reader.

use crate::cipher::aead::{GCM_TAG_LEN, StreamHeader, StreamKey, segment_nonce};
use crate::error::AeadError;
use aes_gcm::AesGcm;
use aes_gcm::aead::array::Array;
use aes_gcm::aead::{AeadCore, AeadInOut, KeyInit, consts::U12};
use std::io::{self, Read, Write};

/// Bounds how far a segment buffer can outrun the bytes that actually arrived.
const SEGMENT_READ_STEP: usize = 64 * 1024;

pub(crate) struct GcmEncryptWriter<W, C>
where
    AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
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
    AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
{
    pub(crate) fn new(writer: W, k_stream: &StreamKey, header: &StreamHeader) -> Self {
        let segment_size = header.segment_size().get() as usize;
        Self {
            w: writer,
            cipher: AesGcm::<C, U12>::new_from_slice(k_stream.as_bytes())
                .expect("32-byte stream key length matches cipher key size"),
            nonce_prefix: header.nonce_prefix,
            segment_size,
            counter: 0,
            // Grown by `write` and reused across segments, so an entry smaller
            // than one segment never reserves a whole segment.
            buf: Vec::new(),
        }
    }

    fn flush_segment(&mut self, is_final: bool) -> io::Result<()> {
        let nonce =
            Array::<u8, U12>::from(segment_nonce(&self.nonce_prefix, self.counter, is_final));
        let tag = self
            .cipher
            .encrypt_inout_detached(&nonce, &[], self.buf.as_mut_slice().into())
            .map_err(|_| io::Error::other("GCM segment encryption failed"))?;
        self.w.write_all(&self.buf)?;
        self.w.write_all(tag.as_slice())?;
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
    AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
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

/// Why the reader stopped.
///
/// Latched so that a later `read` cannot resume a segment whose bytes were
/// already consumed. `AeadError` alone cannot carry the second case: a failure
/// that came from the source is not one of the classes a decoder is meant to
/// report about the datastream.
enum Stopped {
    Aead(AeadError),
    /// The inner reader or an allocation failed partway through a segment. Those
    /// bytes leave with `read_segment`'s local buffer, so the framing cannot be
    /// picked up again — re-reporting the failure is the only honest answer.
    Source(io::ErrorKind, String),
}

impl Stopped {
    fn to_error(&self) -> io::Error {
        match self {
            Self::Aead(e) => e.clone().into(),
            Self::Source(kind, message) => io::Error::new(*kind, message.clone()),
        }
    }
}

/// STREAM-based GCM segment decryption reader.
///
/// Reads one byte ahead so that a full-sized segment is decrypted with the
/// final flag set only when no datastream bytes follow it. Only tag-verified
/// plaintext is ever returned to the caller.
pub(crate) struct GcmDecryptReader<R, C>
where
    AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
{
    r: R,
    cipher: AesGcm<C, U12>,
    nonce_prefix: [u8; 7],
    segment_size: usize,
    counter: u32,
    lookahead: Option<u8>,
    plain: Vec<u8>,
    pos: usize,
    done: bool,
    fuse: Option<Stopped>,
}

impl<R, C> GcmDecryptReader<R, C>
where
    R: Read,
    AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
{
    pub(crate) fn new(reader: R, k_stream: &StreamKey, header: &StreamHeader) -> Self {
        Self {
            r: reader,
            cipher: AesGcm::<C, U12>::new_from_slice(k_stream.as_bytes())
                .expect("32-byte stream key length matches cipher key size"),
            nonce_prefix: header.nonce_prefix,
            segment_size: header.segment_size().get() as usize,
            counter: 0,
            lookahead: None,
            plain: Vec::new(),
            pos: 0,
            done: false,
            fuse: None,
        }
    }

    fn fail(&mut self, e: AeadError) -> io::Error {
        self.fuse = Some(Stopped::Aead(e.clone()));
        e.into()
    }

    fn stop_on_source(&mut self, e: io::Error) -> io::Error {
        self.fuse = Some(Stopped::Source(e.kind(), e.to_string()));
        e
    }

    fn grow_segment_buffer(
        &mut self,
        buf: &mut Vec<u8>,
        filled: usize,
        limit: usize,
    ) -> io::Result<()> {
        let grown = filled + (limit - filled).min(SEGMENT_READ_STEP);
        if grown > buf.capacity() {
            let doubled = buf.capacity().saturating_mul(2);
            let target_capacity = limit.min(doubled.max(grown));
            buf.try_reserve_exact(target_capacity - buf.len())
                .map_err(|_| {
                    self.stop_on_source(io::Error::new(
                        io::ErrorKind::OutOfMemory,
                        format!("failed to allocate {target_capacity} bytes for segment"),
                    ))
                })?;
        }
        buf.resize(grown, 0);
        Ok(())
    }

    fn read_one(&mut self) -> io::Result<Option<u8>> {
        let mut byte = [0u8; 1];
        loop {
            match self.r.read(&mut byte) {
                Ok(0) => return Ok(None),
                Ok(_) => return Ok(Some(byte[0])),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(self.stop_on_source(e)),
            }
        }
    }

    fn read_segment(&mut self, mut buf: Vec<u8>) -> io::Result<(Vec<u8>, bool)> {
        // The segment size comes from the unauthenticated stream header, so
        // committing it up front would let a hostile archive turn a few hundred
        // bytes into a 64 MiB allocation before any tag is verified. Growing in
        // bounded steps keeps committed memory proportional to the bytes that
        // actually arrived, and the geometric growth is clamped to that limit so
        // the tag cannot trigger one final doubling. `try_reserve_exact` reports
        // exhaustion instead of aborting.
        let limit = self.segment_size + GCM_TAG_LEN;
        buf.clear();
        if let Some(byte) = self.lookahead.take() {
            buf.push(byte);
        }
        let mut filled = buf.len();
        while filled < limit {
            if filled == buf.len() {
                self.grow_segment_buffer(&mut buf, filled, limit)?;
            }
            match self.r.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(self.stop_on_source(e)),
            }
        }
        buf.truncate(filled);
        let lookahead = self.read_one()?;
        self.lookahead = lookahead;
        Ok((buf, lookahead.is_none()))
    }

    fn refill(&mut self) -> io::Result<()> {
        let segment = std::mem::take(&mut self.plain);
        let (mut segment, is_final) = self.read_segment(segment)?;
        let i = self.counter;
        // Checked before the short-tail case below so that a segment with bytes
        // after it is reported as the layout violation it is, whatever its length.
        if !is_final && segment.len() < self.segment_size + GCM_TAG_LEN {
            return Err(self.fail(AeadError::Malformed(
                "non-final segment shorter than segment size",
            )));
        }
        if segment.len() < GCM_TAG_LEN {
            // Zero segments is a structural violation (no empty final segment
            // was even present); a short tail after earlier segments is a cut.
            let f = if i == 0 {
                AeadError::Malformed("datastream shorter than one empty final segment")
            } else {
                AeadError::Truncation
            };
            return Err(self.fail(f));
        }
        self.decrypt_in_place(i, is_final, &mut segment)?;
        if is_final {
            self.done = true;
        } else {
            self.counter = self
                .counter
                .checked_add(1)
                .ok_or_else(|| self.fail(AeadError::Malformed("segment counter overflow")))?;
        }
        self.plain = segment;
        self.pos = 0;
        Ok(())
    }

    fn decrypt_in_place(
        &mut self,
        counter: u32,
        is_final: bool,
        segment: &mut Vec<u8>,
    ) -> io::Result<()> {
        let nonce = Array::<u8, U12>::from(segment_nonce(&self.nonce_prefix, counter, is_final));
        match self.cipher.decrypt_in_place(&nonce, &[], segment) {
            Ok(()) => Ok(()),
            Err(_) => Err(self.fail(AeadError::AuthenticationFailure)),
        }
    }
}

impl<R, C> Read for GcmDecryptReader<R, C>
where
    R: Read,
    AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(stopped) = &self.fuse {
            return Err(stopped.to_error());
        }
        // Refilling would consume and decrypt a segment that the caller has no
        // room for, so a zero-length probe would report `Ok(0)` with data still
        // pending — the same guard `ChainReader` carries.
        if buf.is_empty() {
            return Ok(0);
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
    use crate::cipher::aead::{KeyConfirmation, SegmentSize};
    use crate::error::AeadError;
    use aes::Aes256;
    use aes_gcm::aead::Aead;
    use camellia::Camellia256;
    use std::io::{Cursor, Read};

    const KEY: StreamKey = StreamKey::from_bytes([7u8; 32]);
    const PREFIX: [u8; 7] = [3u8; 7];
    const SEG: u32 = 4;

    fn header(segment_size: u32) -> StreamHeader {
        StreamHeader::new(
            [0u8; 32],
            PREFIX,
            SegmentSize::new(segment_size).unwrap(),
            KeyConfirmation::from_bytes([0u8; 32]),
        )
    }

    fn encrypt_all<C>(plain: &[u8]) -> Vec<u8>
    where
        AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
    {
        let mut w = GcmEncryptWriter::<_, C>::new(Vec::new(), &KEY, &header(SEG));
        w.write_all(plain).unwrap();
        w.finish().unwrap()
    }

    fn encrypt_byte_by_byte<C>(plain: &[u8]) -> Vec<u8>
    where
        AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
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
        let cipher = AesGcm::<C, U12>::new_from_slice(KEY.as_bytes()).unwrap();
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

    #[derive(Default)]
    struct RecordingWriter {
        bytes: Vec<u8>,
        write_sizes: Vec<usize>,
    }

    impl Write for RecordingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.write_sizes.push(buf.len());
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn encryption_writes_in_place_buffer_and_detached_tag() {
        let mut writer =
            GcmEncryptWriter::<_, Aes256>::new(RecordingWriter::default(), &KEY, &header(SEG));
        writer.write_all(b"abcd").unwrap();
        let output = writer.finish().unwrap();

        assert_eq!(output.write_sizes, [SEG as usize, GCM_TAG_LEN]);
        assert_eq!(
            decrypt_stream::<Aes256>(SEG as usize, &output.bytes),
            b"abcd"
        );
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
    fn camellia_segments_decrypt_with_the_derived_nonces() {
        let plain = b"abcdefgh";
        let ct = encrypt_all::<Camellia256>(plain);
        assert_eq!(
            decrypt_stream::<Camellia256>(plain.len(), &ct).as_slice(),
            plain
        );
    }

    fn decrypt_all<C>(ciphertext: Vec<u8>) -> io::Result<Vec<u8>>
    where
        AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
    {
        let mut r = GcmDecryptReader::<_, C>::new(Cursor::new(ciphertext), &KEY, &header(SEG));
        let mut out = Vec::new();
        r.read_to_end(&mut out)?;
        Ok(out)
    }

    fn roundtrip<C>(plain: &[u8])
    where
        AesGcm<C, U12>: KeyInit + AeadInOut + AeadCore<NonceSize = U12>,
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

    /// Fails once with a non-`Interrupted` error partway through the stream,
    /// then keeps serving bytes — a source that hiccups rather than dies.
    struct FailingOnceReader<R> {
        inner: R,
        remaining_before_failure: usize,
        armed: bool,
    }

    impl<R: Read> Read for FailingOnceReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.armed && self.remaining_before_failure == 0 {
                self.armed = false;
                return Err(io::Error::other("source hiccup"));
            }
            let n = buf.len().min(self.remaining_before_failure.max(1));
            let n = self.inner.read(&mut buf[..n])?;
            self.remaining_before_failure = self.remaining_before_failure.saturating_sub(n);
            Ok(n)
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
    fn zero_length_read_leaves_the_stream_untouched() {
        let ct = encrypt_all::<Aes256>(b"abcdefgh");
        let mut r = GcmDecryptReader::<_, Aes256>::new(Cursor::new(ct), &KEY, &header(SEG));

        assert_eq!(r.read(&mut []).unwrap(), 0);

        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out, b"abcdefgh");
    }

    /// A segment of exactly `SEGMENT_READ_STEP` needs a second growth step for
    /// its tag alone, which is where unclamped doubling would commit twice the
    /// ciphertext a segment can hold.
    #[test]
    fn segment_capacity_is_clamped_to_the_ciphertext_limit() {
        let stream = header(SEGMENT_READ_STEP as u32);
        let plain = vec![0u8; SEGMENT_READ_STEP + 1];
        let mut w = GcmEncryptWriter::<_, Aes256>::new(Vec::new(), &KEY, &stream);
        w.write_all(&plain).unwrap();
        let ct = w.finish().unwrap();

        let mut r = GcmDecryptReader::<_, Aes256>::new(Cursor::new(ct), &KEY, &stream);
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out, plain);

        let limit = SEGMENT_READ_STEP + GCM_TAG_LEN;
        assert!(
            r.plain.capacity() <= limit,
            "segment capacity {} should be clamped to {limit}",
            r.plain.capacity()
        );
    }

    #[test]
    fn decrypts_with_one_byte_lookahead_and_reuses_the_segment_buffer() {
        let ct = encrypt_all::<Aes256>(b"abcdefgh");
        let mut r = GcmDecryptReader::<_, Aes256>::new(Cursor::new(ct), &KEY, &header(SEG));
        let mut first = [0u8; SEG as usize];

        assert_eq!(r.read(&mut first).unwrap(), first.len());
        assert_eq!(&first, b"abcd");
        assert_eq!(
            r.r.position() as usize,
            SEG as usize + GCM_TAG_LEN + 1,
            "only one byte of the next segment should be consumed"
        );
        let segment_ptr = r.plain.as_ptr();

        let mut second = [0u8; SEG as usize];
        assert_eq!(r.read(&mut second).unwrap(), second.len());
        assert_eq!(&second, b"efgh");
        assert_eq!(
            r.plain.as_ptr(),
            segment_ptr,
            "the consumed plaintext allocation should hold the next segment"
        );
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

    /// The segment framing is generic over the cipher, and Camellia matches AES
    /// in block, nonce and tag size, so the payload shapes above do not need a
    /// second run per cipher — only the instantiation does.
    #[test]
    fn roundtrip_two_segments_camellia() {
        roundtrip::<Camellia256>(b"abcdefgh");
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

    /// Bytes after a segment make it a layout violation rather than a cut end,
    /// even when the segment is too short to hold a tag at all. Distinguishable
    /// from [`AeadError::Truncation`] only once a segment has been verified, so
    /// the first stall carries a whole valid segment plus its look-ahead byte.
    #[test]
    fn non_final_segment_shorter_than_a_tag_is_malformed() {
        let ct = encrypt_all::<Aes256>(b"abcdefgh");
        let first = SEG as usize + GCM_TAG_LEN;
        let reader = StallReader {
            segments: vec![ct[..first + 1].to_vec(), vec![0u8; 1], vec![0u8; 1]],
            index: 0,
            pos: 0,
        };
        let mut r = GcmDecryptReader::<_, Aes256>::new(reader, &KEY, &header(SEG));
        let mut out = [0u8; SEG as usize];

        assert_eq!(r.read(&mut out).unwrap(), out.len());
        assert_eq!(&out, b"abcd");

        let err = r.read(&mut out).unwrap_err();
        assert!(matches!(classify(&err), AeadError::Malformed(_)), "{err}");
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
    fn a_source_error_is_not_reported_as_an_authentication_failure() {
        let ct = encrypt_all::<Aes256>(b"abcdefgh");
        let reader = FailingOnceReader {
            inner: Cursor::new(ct),
            remaining_before_failure: 6,
            armed: true,
        };
        let mut r = GcmDecryptReader::<_, Aes256>::new(reader, &KEY, &header(SEG));
        let mut out = [0u8; 8];

        let first = r.read(&mut out).unwrap_err();
        assert_eq!(first.kind(), io::ErrorKind::Other, "{first}");

        // Not `InvalidData`, which is what every `AeadError` converts to, and
        // still the source's own failure rather than a fresh one.
        let second = r.read(&mut out).unwrap_err();
        assert_eq!(second.kind(), io::ErrorKind::Other, "{second}");
        assert!(second.to_string().contains("source hiccup"), "{second}");
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
