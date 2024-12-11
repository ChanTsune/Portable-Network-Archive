use cipher::{
    typenum::{IsLess, Le, NonZero, U256},
    BlockSizeUser, KeyIvInit, StreamCipher, StreamCipherCoreWrapper,
};
use std::io::{self, Write};

pub(crate) struct StreamCipherWriter<W, T>
where
    T: BlockSizeUser,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
{
    w: W,
    cipher: StreamCipherCoreWrapper<T>,
}

impl<W, T> StreamCipherWriter<W, T>
where
    W: Write,
    T: BlockSizeUser,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
    StreamCipherCoreWrapper<T>: KeyIvInit,
{
    pub(crate) fn new(w: W, key: &[u8], iv: &[u8]) -> io::Result<Self> {
        Ok(Self {
            w,
            cipher: StreamCipherCoreWrapper::<T>::new_from_slices(key, iv)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        })
    }

    pub(crate) fn finish(self) -> io::Result<W> {
        Ok(self.w)
    }
}

impl<W, T> Write for StreamCipherWriter<W, T>
where
    W: Write,
    T: BlockSizeUser,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
    StreamCipherCoreWrapper<T>: StreamCipher,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut buf = buf.to_vec();
        self.cipher.apply_keystream(&mut buf);
        self.w.write(&buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctr::CtrCore;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    type CtrWriter<W, C, F> = StreamCipherWriter<W, CtrCore<C, F>>;
    type Aes128Ctr64LEWriter<W> = CtrWriter<W, aes::Aes128, ctr::flavors::Ctr64LE>;

    #[test]
    fn write_aes128_ctr64le() {
        let key = [0x42u8; 16];
        let iv = [0x24u8; 16];
        let plaintext = *b"hello world! this is my plaintext.";
        let ciphertext = [
            51, 87, 18, 30, 187, 90, 41, 70, 139, 216, 97, 70, 117, 150, 206, 61, 165, 155, 222,
            228, 45, 204, 6, 20, 222, 169, 85, 54, 141, 138, 93, 192, 202, 212,
        ];
        // encrypt in-place
        let mut buf = [0u8; 34];
        let mut cipher = Aes128Ctr64LEWriter::new(buf.as_mut_slice(), &key, &iv).unwrap();
        cipher.write_all(&plaintext).unwrap();

        assert_eq!(buf[..], ciphertext[..]);

        // CTR mode can be used with streaming messages
        let mut out_buf = [0u8; 34];
        let mut cipher = Aes128Ctr64LEWriter::new(out_buf.as_mut_slice(), &key, &iv).unwrap();
        for chunk in buf.chunks_mut(3) {
            cipher.write_all(chunk).unwrap();
        }
        assert_eq!(out_buf[..], plaintext[..]);
    }
}
