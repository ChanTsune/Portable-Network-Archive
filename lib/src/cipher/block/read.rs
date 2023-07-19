use cipher::block_padding::Padding;
use cipher::{Block, BlockCipher, BlockDecryptMut, BlockSizeUser, KeyIvInit};
use std::io::{self, Read};
use std::marker::PhantomData;
use zstd::zstd_safe::WriteBuf;

pub(crate) struct CbcBlockCipherDecryptReader<R, C, P>
where
    R: Read,
    C: BlockDecryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
{
    r: R,
    c: cbc::Decryptor<C>,
    padding: PhantomData<P>,
    remaining: Vec<u8>,
    buf: Vec<u8>,
    eof: bool,
}

impl<R, C, P> CbcBlockCipherDecryptReader<R, C, P>
where
    R: Read,
    C: BlockDecryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
    cbc::Decryptor<C>: KeyIvInit,
{
    pub(crate) fn new_with_iv(r: R, key: &[u8], iv: &[u8]) -> io::Result<Self> {
        Ok(Self {
            r,
            c: cbc::Decryptor::<C>::new_from_slices(key, iv).unwrap(),
            padding: PhantomData,
            remaining: Vec::new(),
            buf: Vec::new(),
            eof: false,
        })
    }

    pub(crate) fn new(mut r: R, key: &[u8]) -> io::Result<Self> {
        let mut iv = vec![0; cbc::Decryptor::<C>::block_size()];
        r.read_exact(&mut iv)?;
        Self::new_with_iv(r, key, &iv)
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
        if self.eof || buf_len == 0 {
            return Ok(0);
        }
        let mut total_written = 0;
        if !self.remaining.is_empty() && buf_len != 0 {
            let l = std::cmp::min(self.remaining.len(), buf_len);
            let d = self.remaining.drain(0..l);
            buf[..l].copy_from_slice(d.as_slice());
            total_written += l;
            if buf_len <= total_written {
                return Ok(total_written);
            }
        }
        let block_size = cbc::Decryptor::<C>::block_size();
        if self.buf.is_empty() {
            let mut prev = vec![0u8; block_size];
            let prev_len = self.r.read(&mut prev)?;
            self.buf = prev;
            if prev_len != block_size {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("Expected buffer size {block_size} but {prev_len}"),
                ));
            }
        }
        loop {
            let mut next = vec![0u8; block_size];
            let next_len = self.r.read(&mut next)?;
            self.eof = next_len == 0;
            let in_block = Block::<cbc::Decryptor<C>>::from_slice(&self.buf);
            let mut out_block = Block::<cbc::Decryptor<C>>::default();
            self.c.decrypt_block_b2b_mut(in_block, &mut out_block);
            let blk = if self.eof {
                P::unpad(&out_block)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                    .as_slice()
            } else {
                out_block.as_slice()
            };
            let should_write_len = std::cmp::min(buf_len - total_written, blk.len());
            let end_of_slice_index = total_written + should_write_len;
            buf[total_written..end_of_slice_index].copy_from_slice(&blk[..should_write_len]);
            total_written += should_write_len;
            self.buf = next;
            if self.eof || buf_len <= total_written {
                self.remaining.extend_from_slice(&blk[should_write_len..]);
                break;
            }
        }
        Ok(total_written)
    }
}

#[cfg(test)]
mod tests {
    use super::CbcBlockCipherDecryptReader;
    use cipher::block_padding::Pkcs7;
    use std::io::Read;

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
        let mut dec = CbcBlockCipherDecryptReader::<_, aes::Aes128, Pkcs7>::new_with_iv(
            ciphertext.as_slice(),
            &key,
            &iv,
        )
        .unwrap();
        for d in buf.chunks_mut(28) {
            dec.read(d).unwrap();
        }
        assert_eq!(buf, plaintext);
    }
}
