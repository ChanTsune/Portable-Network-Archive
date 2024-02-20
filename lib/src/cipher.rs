mod block;
mod stream;

use crate::io::TryIntoInner;
use aes::Aes256;
use camellia::Camellia256;
use cipher::block_padding::Pkcs7;
use ctr::{flavors::Ctr128BE, CtrCore};
use std::io::{self, Read, Write};

type CtrReader<R, C, F> = stream::StreamCipherReader<R, CtrCore<C, F>>;
type CtrWriter<W, C, F> = stream::StreamCipherWriter<W, CtrCore<C, F>>;
pub(crate) type Ctr128BEReader<R, C> = CtrReader<R, C, Ctr128BE>;
pub(crate) type Ctr128BEWriter<W, C> = CtrWriter<W, C, Ctr128BE>;

pub(crate) type EncryptCbcAes256Writer<W> = block::CbcBlockCipherEncryptWriter<W, Aes256, Pkcs7>;
pub(crate) type DecryptCbcAes256Reader<R> = block::CbcBlockCipherDecryptReader<R, Aes256, Pkcs7>;
pub(crate) type EncryptCbcCamellia256Writer<W> =
    block::CbcBlockCipherEncryptWriter<W, Camellia256, Pkcs7>;
pub(crate) type DecryptCbcCamellia256Reader<R> =
    block::CbcBlockCipherDecryptReader<R, Camellia256, Pkcs7>;

pub(crate) enum CipherWriter<W: Write> {
    No(W),
    CbcAes(EncryptCbcAes256Writer<W>),
    CbcCamellia(EncryptCbcCamellia256Writer<W>),
    CtrAes(Ctr128BEWriter<W, Aes256>),
    CtrCamellia(Ctr128BEWriter<W, Camellia256>),
}

impl<W: Write> Write for CipherWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::No(w) => w.write(buf),
            Self::CbcAes(w) => w.write(buf),
            Self::CbcCamellia(w) => w.write(buf),
            Self::CtrAes(w) => w.write(buf),
            Self::CtrCamellia(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::No(w) => w.flush(),
            Self::CbcAes(w) => w.flush(),
            Self::CbcCamellia(w) => w.flush(),
            Self::CtrAes(w) => w.flush(),
            Self::CtrCamellia(w) => w.flush(),
        }
    }
}

impl<W: Write> TryIntoInner<W> for CipherWriter<W> {
    fn try_into_inner(self) -> io::Result<W> {
        match self {
            Self::No(w) => Ok(w),
            Self::CbcAes(w) => w.finish(),
            Self::CbcCamellia(w) => w.finish(),
            Self::CtrAes(w) => w.finish(),
            Self::CtrCamellia(w) => w.finish(),
        }
    }
}

pub(crate) enum DecryptReader<R: Read> {
    No(R),
    CbcAes(DecryptCbcAes256Reader<R>),
    CbcCamellia(DecryptCbcCamellia256Reader<R>),
    CtrAes(Ctr128BEReader<R, Aes256>),
    CtrCamellia(Ctr128BEReader<R, Camellia256>),
}

impl<R: Read> Read for DecryptReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            DecryptReader::No(r) => r.read(buf),
            DecryptReader::CbcAes(r) => r.read(buf),
            DecryptReader::CbcCamellia(r) => r.read(buf),
            DecryptReader::CtrAes(r) => r.read(buf),
            DecryptReader::CtrCamellia(r) => r.read(buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use aes::Aes256;
    use camellia::Camellia256;
    use cipher::{
        block_padding::Pkcs7, BlockCipher, BlockDecryptMut, BlockEncryptMut, BlockSizeUser,
        KeyIvInit, KeySizeUser,
    };
    use std::io;

    fn encrypt_cbc<Cipher>(key: &[u8], iv: &[u8], data: &[u8]) -> io::Result<Vec<u8>>
    where
        Cipher: BlockEncryptMut + BlockCipher,
        cbc::Encryptor<Cipher>: KeyIvInit,
    {
        let encryptor = cbc::Encryptor::<Cipher>::new_from_slices(key, iv)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut d = encryptor.encrypt_padded_vec_mut::<Pkcs7>(data);
        let mut e = Vec::from(iv);
        e.append(&mut d);
        Ok(e)
    }

    fn decrypt_cbc<Cipher>(key: &[u8], data: &[u8]) -> io::Result<Vec<u8>>
    where
        Cipher: BlockDecryptMut + BlockCipher,
        cbc::Decryptor<Cipher>: KeyIvInit,
    {
        let decryptor =
            cbc::Decryptor::<Cipher>::new_from_slices(key, &data[0..Cipher::block_size()])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let data = decryptor
            .decrypt_padded_vec_mut::<Pkcs7>(&data[Cipher::block_size()..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(data)
    }

    pub(crate) fn encrypt_aes256_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> io::Result<Vec<u8>> {
        encrypt_cbc::<Aes256>(key, iv, data)
    }

    pub(crate) fn decrypt_aes256_cbc(key: &[u8], data: &[u8]) -> io::Result<Vec<u8>> {
        decrypt_cbc::<Aes256>(key, data)
    }

    pub(crate) fn encrypt_camellia256_cbc(
        key: &[u8],
        iv: &[u8],
        data: &[u8],
    ) -> io::Result<Vec<u8>> {
        encrypt_cbc::<Camellia256>(key, iv, data)
    }

    pub(crate) fn decrypt_camellia256_cbc(key: &[u8], data: &[u8]) -> io::Result<Vec<u8>> {
        decrypt_cbc::<Camellia256>(key, data)
    }

    #[test]
    fn aes() {
        let key = vec![0; Aes256::key_size()];
        let iv = vec![0; Aes256::block_size()];
        let plain_text = b"plain";
        let encrypted_text = encrypt_aes256_cbc(&key, &iv, plain_text).unwrap();

        let decrypted_text = decrypt_aes256_cbc(&key, &encrypted_text).unwrap();

        assert_eq!(plain_text.as_slice(), decrypted_text.as_slice())
    }

    #[test]
    fn camellia() {
        let key = vec![0; Camellia256::key_size()];
        let iv = vec![0; Camellia256::block_size()];
        let plain_text = b"plain";
        let encrypted_text = encrypt_camellia256_cbc(&key, &iv, plain_text).unwrap();

        let decrypted_text = decrypt_camellia256_cbc(&key, &encrypted_text).unwrap();

        assert_eq!(plain_text.as_slice(), decrypted_text.as_slice())
    }
}
