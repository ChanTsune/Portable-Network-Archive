use arrayvec::ArrayVec;
use cipher::block_padding::Padding;
use cipher::{Block, BlockCipher, BlockEncryptMut, BlockSizeUser, KeyIvInit};
use std::io::{self, Write};
use std::marker::PhantomData;

pub(crate) struct CbcBlockCipherEncryptWriter<W, C, P>
where
    C: BlockEncryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
{
    w: W,
    c: cbc::Encryptor<C>,
    padding: PhantomData<P>,
    buf: ArrayVec<u8, 16>,
}

impl<W, C, P> CbcBlockCipherEncryptWriter<W, C, P>
where
    W: Write,
    C: BlockEncryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
    cbc::Encryptor<C>: KeyIvInit,
{
    pub(crate) fn new(w: W, key: &[u8], iv: &[u8]) -> io::Result<Self> {
        debug_assert_eq!(cbc::Encryptor::<C>::block_size(), 16);
        Ok(Self {
            w,
            c: cbc::Encryptor::<C>::new_from_slices(key, iv)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
            padding: PhantomData,
            buf: ArrayVec::new(),
        })
    }
}

impl<W, C, P> CbcBlockCipherEncryptWriter<W, C, P>
where
    W: Write,
    C: BlockEncryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
{
    fn encrypt_write_with_padding(mut self) -> io::Result<W> {
        let pos = self.buf.len();
        unsafe { self.buf.set_len(cbc::Encryptor::<C>::block_size()) };
        let block = Block::<cbc::Encryptor<C>>::from_mut_slice(&mut self.buf);
        P::pad(block, pos);
        self.c.encrypt_block_inout_mut(block.into());
        self.w.write_all(block.as_slice())?;
        Ok(self.w)
    }
}

impl<W, C, P> Write for CbcBlockCipherEncryptWriter<W, C, P>
where
    W: Write,
    C: BlockEncryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let block_size = cbc::Encryptor::<C>::block_size();
        if buf.len() + self.buf.len() < block_size {
            self.buf
                .try_extend_from_slice(buf)
                .expect("buffer capacity exceeded: crypto block size invariant violated");
            return Ok(buf.len());
        }
        let mut total_written = 0;
        if !self.buf.is_empty() {
            let remaining = block_size - self.buf.len();
            self.buf
                .try_extend_from_slice(&buf[..remaining])
                .expect("buffer capacity exceeded: crypto block size invariant violated");
            total_written += remaining;

            let inout_block = Block::<cbc::Encryptor<C>>::from_mut_slice(&mut self.buf);
            self.c.encrypt_block_inout_mut(inout_block.into());
            self.w.write_all(inout_block.as_slice())?;
            self.buf.clear();
        }

        let mut out_block = Block::<cbc::Encryptor<C>>::default();
        let chunks = buf[total_written..].chunks_exact(block_size);
        let remainder = chunks.remainder();
        for b in chunks {
            let in_block = Block::<cbc::Encryptor<C>>::from_slice(b);
            self.c.encrypt_block_b2b_mut(in_block, &mut out_block);
            self.w.write_all(out_block.as_slice())?;
            total_written += b.len();
        }
        self.buf
            .try_extend_from_slice(remainder)
            .expect("buffer capacity exceeded: crypto block size invariant violated");
        total_written += remainder.len();
        Ok(total_written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

impl<W, C, P> CbcBlockCipherEncryptWriter<W, C, P>
where
    W: Write,
    C: BlockEncryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
{
    pub(crate) fn finish(self) -> io::Result<W> {
        self.encrypt_write_with_padding()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cipher::block_padding::Pkcs7;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn write_encrypt() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext = *b"hello world! this is my plaintext.";
        let ciphertext = [
            199u8, 254, 36, 126, 249, 123, 33, 240, 124, 189, 210, 108, 181, 211, 70, 191, 210,
            120, 103, 203, 0, 217, 72, 103, 35, 225, 89, 151, 143, 185, 165, 249, 20, 207, 178, 40,
            167, 16, 222, 65, 113, 227, 150, 231, 182, 207, 133, 158,
        ];

        let mut writer =
            CbcBlockCipherEncryptWriter::<_, aes::Aes128, Pkcs7>::new(Vec::new(), &key, &iv)
                .unwrap();
        for p in plaintext.chunks(7) {
            writer.write_all(p).unwrap();
        }
        let ct = writer.finish().unwrap();
        assert_eq!(&ct[..], &ciphertext[..]);
    }

    #[test]
    fn write_len() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext = b"hello world! this is my plaintext.".repeat(1024);
        let mut writer =
            CbcBlockCipherEncryptWriter::<_, aes::Aes128, Pkcs7>::new(Vec::new(), &key, &iv)
                .unwrap();
        for p in plaintext.chunks(13) {
            assert_eq!(writer.write(p).unwrap(), p.len());
        }
        writer.finish().unwrap();
    }
}
