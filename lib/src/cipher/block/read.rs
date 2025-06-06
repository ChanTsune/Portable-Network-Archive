use arrayvec::ArrayVec;
use cipher::block_padding::Padding;
use cipher::{Block, BlockCipher, BlockDecryptMut, BlockSizeUser, KeyIvInit};
use std::io::{self, Read};
use std::marker::PhantomData;

pub(crate) struct CbcBlockCipherDecryptReader<R, C, P>
where
    C: BlockDecryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
{
    r: R,
    c: cbc::Decryptor<C>,
    padding: PhantomData<P>,
    remaining: ArrayVec<u8, 16>,
    buf: ArrayVec<u8, 16>,
    eof: bool,
}

impl<R, C, P> CbcBlockCipherDecryptReader<R, C, P>
where
    R: Read,
    C: BlockDecryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
    cbc::Decryptor<C>: KeyIvInit,
{
    pub(crate) fn new(mut r: R, key: &[u8], iv: &[u8]) -> io::Result<Self> {
        let block_size = cbc::Decryptor::<C>::block_size();
        let mut buf = ArrayVec::new();
        debug_assert_eq!(block_size, buf.capacity());
        unsafe { buf.set_len(buf.capacity()) };
        let prev_len = r.read(&mut buf)?;
        if prev_len != block_size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("Expected buffer size {block_size} but {prev_len}"),
            ));
        }
        Ok(Self {
            r,
            c: cbc::Decryptor::<C>::new_from_slices(key, iv).unwrap(),
            padding: PhantomData,
            remaining: ArrayVec::new(),
            buf,
            eof: false,
        })
    }
}

impl<R, C, P> Read for CbcBlockCipherDecryptReader<R, C, P>
where
    R: Read,
    C: BlockDecryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let buf_len = buf.len();
        if buf_len == 0 {
            return Ok(0);
        }
        let mut total_written = 0;
        if !self.remaining.is_empty() && buf_len != 0 {
            let l = std::cmp::min(self.remaining.len(), buf_len);
            buf[..l].copy_from_slice(&self.remaining[..l]);
            self.remaining.drain(..l);
            total_written += l;
            if buf_len <= total_written {
                return Ok(total_written);
            }
        }
        if self.eof {
            return Ok(0);
        }
        let block_size = cbc::Decryptor::<C>::block_size();
        let mut out_block = Block::<cbc::Decryptor<C>>::default();
        for chunk in buf[total_written..].chunks_mut(block_size) {
            let in_block = Block::<cbc::Decryptor<C>>::from_slice(&self.buf);
            self.c.decrypt_block_b2b_mut(in_block, &mut out_block);
            let next_len = self.r.read(&mut self.buf)?;
            self.eof = next_len == 0;
            let blk = if self.eof {
                P::unpad(&out_block).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            } else {
                out_block.as_slice()
            };
            let should_write_len = std::cmp::min(chunk.len(), blk.len());
            chunk[..should_write_len].copy_from_slice(&blk[..should_write_len]);
            total_written += should_write_len;
            if self.eof || buf_len <= total_written {
                self.remaining
                    .try_extend_from_slice(&blk[should_write_len..])
                    .expect("");
                break;
            }
        }
        Ok(total_written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cipher::block_padding::Pkcs7;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

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
}
