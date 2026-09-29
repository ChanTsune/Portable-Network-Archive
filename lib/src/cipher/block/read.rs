//! CBC block cipher decryption reader.

use arrayvec::ArrayVec;
use cipher::block_padding::Padding;
use cipher::{Block, BlockCipherDecrypt, BlockModeDecrypt, BlockSizeUser, InOutBuf, KeyIvInit};
use std::io::{self, Read};
use std::marker::PhantomData;

use crate::util::io::read_to_fill;

const DECRYPT_BATCH_BLOCKS: usize = 32;

pub(crate) struct CbcBlockCipherDecryptReader<R, C, P>
where
    C: BlockCipherDecrypt,
    cbc::Decryptor<C>: BlockModeDecrypt,
    P: Padding,
{
    r: R,
    c: cbc::Decryptor<C>,
    padding: PhantomData<P>,
    remaining: ArrayVec<u8, 16>,
    buf: ArrayVec<u8, 16>,
    eof: bool,
    failed: bool,
}

impl<R, C, P> CbcBlockCipherDecryptReader<R, C, P>
where
    R: Read,
    C: BlockCipherDecrypt,
    cbc::Decryptor<C>: BlockModeDecrypt,
    P: Padding,
    cbc::Decryptor<C>: KeyIvInit,
{
    pub(crate) fn new(mut r: R, key: &[u8], iv: &[u8]) -> io::Result<Self> {
        let block_size = cbc::Decryptor::<C>::block_size();
        let mut buf = ArrayVec::new();
        debug_assert_eq!(block_size, buf.capacity());
        unsafe { buf.set_len(buf.capacity()) };
        r.read_exact(&mut buf)?;
        Ok(Self {
            r,
            c: cbc::Decryptor::<C>::new_from_slices(key, iv)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
            padding: PhantomData,
            remaining: ArrayVec::new(),
            buf,
            eof: false,
            failed: false,
        })
    }
}

impl<R, C, P> CbcBlockCipherDecryptReader<R, C, P>
where
    C: BlockCipherDecrypt,
    cbc::Decryptor<C>: BlockModeDecrypt,
    P: Padding,
{
    fn fail(&mut self, e: io::Error) -> io::Error {
        self.failed = true;
        e
    }
}

impl<R, C, P> Read for CbcBlockCipherDecryptReader<R, C, P>
where
    R: Read,
    C: BlockCipherDecrypt,
    cbc::Decryptor<C>: BlockModeDecrypt,
    P: Padding,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.failed {
            return Err(io::Error::other("CBC reader failed after a previous error"));
        }
        let buf_len = buf.len();
        if buf_len == 0 {
            return Ok(0);
        }
        let block_size = cbc::Decryptor::<C>::block_size();
        debug_assert_eq!(block_size, 16);
        let mut total_written = 0;
        if !self.remaining.is_empty() {
            let l = std::cmp::min(self.remaining.len(), buf_len);
            buf[..l].copy_from_slice(&self.remaining[..l]);
            self.remaining.drain(..l);
            total_written += l;
            if buf_len <= total_written {
                return Ok(total_written);
            }
        }
        if self.eof {
            return Ok(total_written);
        }
        // (Batch + 1) blocks: the pending holdback plus fresh blocks.
        let mut staging = [0u8; (DECRYPT_BATCH_BLOCKS + 1) * 16];
        while total_written < buf_len && !self.eof {
            let space_blocks = (buf_len - total_written).div_ceil(block_size);
            let want_new = space_blocks.min(DECRYPT_BATCH_BLOCKS);
            // Bulk-fill up to `want_new` fresh blocks; a short fill means
            // EOF was reached on a block boundary.
            let cap = want_new * block_size;
            let filled = match read_to_fill(&mut self.r, &mut staging[block_size..block_size + cap])
            {
                Ok(filled) => filled,
                Err(e) => return Err(self.fail(e)),
            };
            if filled % block_size != 0 {
                return Err(self.fail(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("expected block of {block_size} bytes, got {filled}"),
                )));
            }
            let fresh = filled / block_size;
            let eof_now = filled < cap;
            // Stash the next holdback before the in-place bulk decrypt.
            let mut next_pending = [0u8; 16];
            if !eof_now {
                next_pending.copy_from_slice(
                    &staging
                        [block_size + (fresh - 1) * block_size..block_size + fresh * block_size],
                );
            }
            staging[..block_size].copy_from_slice(&self.buf);
            let total_blocks = 1 + fresh;
            // Decrypt every block exactly once: the held-back block stays
            // ciphertext (and the cipher IV stays behind it) until the next
            // batch proves a byte follows it. Only a proven-final batch
            // decrypts through its last block.
            let decrypt_blocks = if eof_now {
                total_blocks
            } else {
                total_blocks - 1
            };
            {
                let slice = &mut staging[..decrypt_blocks * block_size];
                let inout = InOutBuf::from(slice);
                let (mut blocks, tail) =
                    inout.into_chunks::<<cbc::Decryptor<C> as BlockSizeUser>::BlockSize>();
                debug_assert!(tail.is_empty());
                self.c.decrypt_blocks_inout(blocks.reborrow());
            }
            let plain_end = if eof_now {
                let last = <&Block<cbc::Decryptor<C>>>::try_from(
                    &staging[(total_blocks - 1) * block_size..total_blocks * block_size],
                )
                .expect("block slicing is exact");
                let unpadded = match P::unpad(last) {
                    Ok(unpadded) => unpadded,
                    Err(e) => {
                        return Err(self.fail(io::Error::new(io::ErrorKind::InvalidData, e)));
                    }
                };
                fresh * block_size + unpadded.len()
            } else {
                self.buf.copy_from_slice(&next_pending);
                (total_blocks - 1) * block_size
            };
            let emit = std::cmp::min(plain_end, buf_len - total_written);
            buf[total_written..total_written + emit].copy_from_slice(&staging[..emit]);
            total_written += emit;
            // Spill is < one block because `want_new` covers the caller's space.
            self.remaining
                .try_extend_from_slice(&staging[emit..plain_end])
                .expect("spillover is at most one block");
            self.eof = eof_now;
        }
        Ok(total_written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::io::tests::PartialReader;
    use cipher::block_padding::Pkcs7;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    fn encrypt_vec(key: &[u8], iv: &[u8], plaintext: &[u8]) -> Vec<u8> {
        use cipher::BlockModeEncrypt;
        cbc::Encryptor::<aes::Aes128>::new_from_slices(key, iv)
            .unwrap()
            .encrypt_padded_vec::<Pkcs7>(plaintext)
    }

    /// Batching is internal: the same ciphertext must decode identically no
    /// matter what buffer sizes the caller reads with, including sizes that
    /// straddle batch and block edges.
    #[test]
    fn read_output_independent_of_caller_buffer_sizes() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
        let ciphertext = encrypt_vec(&key, &iv, &plaintext);
        for size in [1usize, 7, 15, 16, 17, 31, 33, 511, 512, 513, 8192] {
            let mut dec = CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(
                ciphertext.as_slice(),
                &key,
                &iv,
            )
            .unwrap();
            let mut out = Vec::new();
            let mut buf = vec![0u8; size];
            loop {
                match dec.read(&mut buf).unwrap() {
                    0 => break,
                    n => out.extend_from_slice(&buf[..n]),
                }
            }
            assert_eq!(out, plaintext, "caller buffer size {size}");
        }
    }

    /// Holdback edges: empty, sub-block, exact-block and exact-batch
    /// plaintexts must all round-trip through the padding logic.
    #[test]
    fn read_exact_block_and_empty_plaintexts() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        for len in [0usize, 1, 15, 16, 17, 32, 512, 513] {
            let plaintext: Vec<u8> = (0..len as u32).map(|i| (i % 251) as u8).collect();
            let ciphertext = encrypt_vec(&key, &iv, &plaintext);
            let mut dec = CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(
                ciphertext.as_slice(),
                &key,
                &iv,
            )
            .unwrap();
            let mut out = Vec::new();
            dec.read_to_end(&mut out).unwrap();
            assert_eq!(out, plaintext, "plaintext length {len}");
        }
    }

    #[test]
    fn read_decrypt() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext = *b"hello world! this is my plaintext.";
        let ciphertext = [
            199u8, 254, 36, 126, 249, 123, 33, 240, 124, 189, 210, 108, 181, 211, 70, 191, 210,
            120, 103, 203, 0, 217, 72, 103, 35, 225, 89, 151, 143, 185, 165, 249, 20, 207, 178, 40,
            167, 16, 222, 65, 113, 227, 150, 231, 182, 207, 133, 158,
        ];

        let mut buf = [0u8; 34];
        let mut dec = CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(
            ciphertext.as_slice(),
            &key,
            &iv,
        )
        .unwrap();
        for d in buf.chunks_mut(28) {
            dec.read_exact(d).unwrap();
        }
        assert_eq!(buf, plaintext);
    }

    #[test]
    fn read_decrypt_errors_on_partial_block() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let ciphertext = [
            199u8, 254, 36, 126, 249, 123, 33, 240, 124, 189, 210, 108, 181, 211, 70, 191, 210,
            120, 103, 203, 0, 217, 72, 103, 35, 225, 89, 151, 143, 185, 165, 249, 20, 207, 178, 40,
            167, 16, 222, 65, 113, 227, 150, 231, 182, 207, 133, 158,
        ];
        let truncated = ciphertext[..24].to_vec();
        let reader = PartialReader::new(truncated, [16u8, 8]);
        let mut dec =
            CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(reader, &key, &iv).unwrap();
        let mut buf = [0u8; 34];
        let err = dec.read(&mut buf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn read_decrypt_partial_reads() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let ciphertext = [
            199u8, 254, 36, 126, 249, 123, 33, 240, 124, 189, 210, 108, 181, 211, 70, 191, 210,
            120, 103, 203, 0, 217, 72, 103, 35, 225, 89, 151, 143, 185, 165, 249, 20, 207, 178, 40,
            167, 16, 222, 65, 113, 227, 150, 231, 182, 207, 133, 158,
        ];
        let plaintext = *b"hello world! this is my plaintext.";
        let chunk_sizes = [5u8, 3, 8, 4, 6, 7, 6, 9];
        let reader = PartialReader::new(ciphertext.to_vec(), chunk_sizes);
        let mut dec =
            CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(reader, &key, &iv).unwrap();

        let mut buf = Vec::new();
        dec.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, plaintext);
    }

    #[test]
    fn read_decrypt_par_1byte() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let ciphertext = [
            199u8, 254, 36, 126, 249, 123, 33, 240, 124, 189, 210, 108, 181, 211, 70, 191, 210,
            120, 103, 203, 0, 217, 72, 103, 35, 225, 89, 151, 143, 185, 165, 249, 20, 207, 178, 40,
            167, 16, 222, 65, 113, 227, 150, 231, 182, 207, 133, 158,
        ];
        let plaintext = *b"hello world! this is my plaintext.";
        let reader = PartialReader::new(ciphertext.to_vec(), std::iter::repeat(1));
        let mut dec =
            CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(reader, &key, &iv).unwrap();

        let mut buf = [0u8; 34];
        for d in buf.chunks_mut(1) {
            dec.read_exact(d).unwrap();
        }
        assert_eq!(buf, plaintext);
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

    /// Fails once, then serves bytes.
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

    #[test]
    fn read_rejects_corrupted_final_block() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext: Vec<u8> = (0..100u32).map(|i| (i % 251) as u8).collect();
        let mut ciphertext = encrypt_vec(&key, &iv, &plaintext);
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0x01;
        let mut dec = CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(
            ciphertext.as_slice(),
            &key,
            &iv,
        )
        .unwrap();
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_errors_stop_reader() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext: Vec<u8> = (0..100u32).map(|i| (i % 251) as u8).collect();
        let mut corrupted = encrypt_vec(&key, &iv, &plaintext);
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0x01;
        let mut dec = CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(
            corrupted.as_slice(),
            &key,
            &iv,
        )
        .unwrap();
        let mut buf = [0u8; 128];
        let first = dec.read(&mut buf).unwrap_err();
        assert_eq!(first.kind(), io::ErrorKind::InvalidData);
        for _ in 0..5 {
            let retry = dec.read(&mut buf).unwrap_err();
            assert_eq!(
                retry.to_string(),
                "CBC reader failed after a previous error"
            );
        }

        let big: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
        let ct = encrypt_vec(&key, &iv, &big);
        let truncated = ct[..40 * 16 + 8].to_vec();
        let mut dec = CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(
            truncated.as_slice(),
            &key,
            &iv,
        )
        .unwrap();
        let mut buf = vec![0u8; 8192];
        let first = dec.read(&mut buf).unwrap_err();
        assert_eq!(first.kind(), io::ErrorKind::UnexpectedEof);
        for _ in 0..5 {
            let retry = dec.read(&mut buf).unwrap_err();
            assert_eq!(
                retry.to_string(),
                "CBC reader failed after a previous error"
            );
        }
    }

    #[test]
    fn read_source_error_stops_reader() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
        let ciphertext = encrypt_vec(&key, &iv, &plaintext);
        let reader = FailingOnceReader {
            inner: ciphertext.as_slice(),
            remaining_before_failure: 600,
            armed: true,
        };
        let mut dec =
            CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(reader, &key, &iv).unwrap();
        let mut buf = vec![0u8; 8192];
        let first = dec.read(&mut buf).unwrap_err();
        assert_eq!(first.kind(), io::ErrorKind::Other);
        assert_eq!(first.to_string(), "source hiccup");
        let second = dec.read(&mut buf).unwrap_err();
        assert_eq!(second.kind(), io::ErrorKind::Other);
        assert_eq!(
            second.to_string(),
            "CBC reader failed after a previous error"
        );
    }

    #[test]
    fn read_multi_batch_survives_partial_and_interrupted_sources() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
        let ciphertext = encrypt_vec(&key, &iv, &plaintext);

        let reader = PartialReader::new(
            ciphertext.clone(),
            [7u8, 3, 1, 16, 5, 9, 2, 13].into_iter().cycle(),
        );
        let mut dec =
            CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(reader, &key, &iv).unwrap();
        let mut out = Vec::new();
        let mut buf = [0u8; 100];
        loop {
            match dec.read(&mut buf).unwrap() {
                0 => break,
                n => out.extend_from_slice(&buf[..n]),
            }
        }
        assert_eq!(out, plaintext);

        let reader = InterruptingReader {
            inner: PartialReader::new(ciphertext.clone(), std::iter::repeat(5)),
            armed: true,
        };
        let mut dec =
            CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(reader, &key, &iv).unwrap();
        let mut out = Vec::new();
        dec.read_to_end(&mut out).unwrap();
        assert_eq!(out, plaintext);
    }

    #[test]
    fn read_truncated_mid_batch_is_unexpected_eof() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
        let ciphertext = encrypt_vec(&key, &iv, &plaintext);
        let truncated = ciphertext[..40 * 16 + 8].to_vec();
        let mut dec = CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(
            truncated.as_slice(),
            &key,
            &iv,
        )
        .unwrap();
        let mut buf = vec![0u8; 8192];
        let err = dec.read(&mut buf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);

        let reader = PartialReader::new(truncated, std::iter::repeat(3));
        let mut dec =
            CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new(reader, &key, &iv).unwrap();
        let mut out = Vec::new();
        let mut one = [0u8; 1];
        let err = loop {
            match dec.read(&mut one) {
                Ok(0) => panic!("truncated stream must not decode to EOF"),
                Ok(_) => out.extend_from_slice(&one),
                Err(e) => break e,
            }
        };
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }
}
