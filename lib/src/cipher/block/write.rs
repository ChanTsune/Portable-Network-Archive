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
    buf: Vec<u8>,
}

impl<W, C, P> CbcBlockCipherEncryptWriter<W, C, P>
where
    W: Write,
    C: BlockEncryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
    cbc::Encryptor<C>: KeyIvInit,
{
    pub(crate) fn new(w: W, key: &[u8], iv: &[u8]) -> io::Result<Self> {
        Ok(Self {
            w,
            c: cbc::Encryptor::<C>::new_from_slices(key, iv).unwrap(),
            padding: PhantomData,
            buf: Vec::new(),
        })
    }
}

impl<W, C, P> CbcBlockCipherEncryptWriter<W, C, P>
where
    W: Write,
    C: BlockEncryptMut + BlockCipher,
    P: Padding<<C as BlockSizeUser>::BlockSize>,
{
    fn encrypt_write_block(
        &mut self,
        block: &Block<cbc::Encryptor<C>>,
        len: usize,
    ) -> io::Result<usize> {
        let mut out_block = Block::<cbc::Encryptor<C>>::default();
        self.c.encrypt_block_b2b_mut(block, &mut out_block);
        self.w.write_all(out_block.as_slice())?;
        Ok(len)
    }

    fn encrypt_write(&mut self, data: &[u8], len: usize) -> io::Result<usize> {
        let in_block = Block::<cbc::Encryptor<C>>::from_slice(data);
        self.encrypt_write_block(in_block, len)
    }

    fn encrypt_write_with_padding(&mut self) -> io::Result<usize> {
        let (mut v, pos) = {
            let d = self.buf.drain(..);
            let pos = d.len();
            let mut v = vec![0; cbc::Encryptor::<C>::block_size()];
            v[..pos].copy_from_slice(d.as_slice());
            (v, pos)
        };
        let block = Block::<cbc::Encryptor<C>>::from_mut_slice(&mut v);
        P::pad(block, pos);
        self.encrypt_write_block(block, pos)
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
            self.buf.extend_from_slice(buf);
            return Ok(buf.len());
        }
        let (vec, remaining) = {
            let d = self.buf.drain(..);
            let remaining = block_size - d.len();
            let mut vec = Vec::with_capacity(block_size);
            vec.extend_from_slice(d.as_slice());
            vec.extend_from_slice(&buf[..remaining]);
            (vec, remaining)
        };
        let mut total_written = self.encrypt_write(&vec, remaining)?;
        for b in buf[remaining..].chunks(block_size) {
            if b.len() == block_size {
                total_written += self.encrypt_write(b, b.len())?;
            } else {
                self.buf.extend_from_slice(b);
                total_written += b.len();
            }
        }
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
    pub(crate) fn finish(mut self) -> io::Result<W> {
        self.encrypt_write_with_padding()?;
        Ok(self.w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cipher::block_padding::Pkcs7;

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

        let ct = {
            let mut writer =
                CbcBlockCipherEncryptWriter::<_, aes::Aes128, Pkcs7>::new(Vec::new(), &key, &iv)
                    .unwrap();
            for p in plaintext.chunks(8) {
                writer.write_all(p).unwrap();
            }
            writer.finish().unwrap()
        };
        assert_eq!(&ct[..], &ciphertext[..]);
    }

    #[test]
    fn write_len() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext = *b"hello world! this is my plaintext.";
        let mut ct = Vec::new();
        {
            let mut writer =
                CbcBlockCipherEncryptWriter::<_, aes::Aes128, Pkcs7>::new(&mut ct, &key, &iv)
                    .unwrap();
            for p in plaintext.chunks(7) {
                assert_eq!(writer.write(p).unwrap(), p.len())
            }
        };
    }
}
