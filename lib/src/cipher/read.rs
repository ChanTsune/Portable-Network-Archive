use cipher::typenum::{IsLess, Le, NonZero, U256};
use cipher::{BlockSizeUser, KeyIvInit, StreamCipher, StreamCipherCoreWrapper};
use ctr::CtrCore;
use std::io::{self, Read};

pub(crate) struct StreamCipherReader<R, T>
where
    R: Read,
    T: BlockSizeUser,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
{
    r: R,
    cipher: StreamCipherCoreWrapper<T>,
}

impl<R, T> StreamCipherReader<R, T>
where
    R: Read,
    T: BlockSizeUser,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
    StreamCipherCoreWrapper<T>: KeyIvInit,
{
    pub(crate) fn new(r: R, key: &[u8], iv: &[u8]) -> Self {
        Self {
            r,
            cipher: StreamCipherCoreWrapper::<T>::new_from_slices(key, iv).unwrap(),
        }
    }
}

impl<R, T> Read for StreamCipherReader<R, T>
where
    R: Read,
    T: BlockSizeUser,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
    StreamCipherCoreWrapper<T>: StreamCipher,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.r.read(buf)?;
        self.cipher.apply_keystream(buf);
        Ok(n)
    }
}

pub(crate) type CtrReader<W, C, F> = StreamCipherReader<W, CtrCore<C, F>>;

#[cfg(test)]
mod tests {
    use super::CtrReader;
    use std::io::Read;

    type Aes128Ctr64LEReader<R> = CtrReader<R, aes::Aes128, ctr::flavors::Ctr64LE>;

    #[test]
    fn read_aes128_ctr64le() {
        let key = [0x42u8; 16];
        let iv = [0x24u8; 16];
        let plaintext = *b"hello world! this is my plaintext.";
        let ciphertext = [
            51, 87, 18, 30, 187, 90, 41, 70, 139, 216, 97, 70, 117, 150, 206, 61, 165, 155, 222,
            228, 45, 204, 6, 20, 222, 169, 85, 54, 141, 138, 93, 192, 202, 212,
        ];
        // encrypt in-place
        let mut buf = [0u8; 34];
        let mut cipher = Aes128Ctr64LEReader::new(plaintext.as_slice(), &key, &iv);
        cipher.read(&mut buf).unwrap();

        assert_eq!(buf[..], ciphertext[..]);

        // CTR mode can be used with streaming messages
        let mut out_buf = [0u8; 34];
        let mut cipher = Aes128Ctr64LEReader::new(buf.as_slice(), &key, &iv);
        for chunk in out_buf.chunks_mut(3) {
            cipher.read_exact(chunk).unwrap();
        }
        assert_eq!(out_buf[..], plaintext[..]);
    }
}
