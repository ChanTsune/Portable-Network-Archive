//! Cipher implementations for PNA archive encryption and decryption.

mod block;
mod stream;

use crate::io::TryIntoInner;
use aes::Aes256;
use camellia::Camellia256;
use cipher::block_padding::Pkcs7;
use ctr::{CtrCore, flavors::Ctr128BE};
use std::io::{self, Read, Write};

/// A type alias for a CTR mode stream cipher reader.
///
/// This type represents a reader that decrypts data using CTR mode with a 128-bit block size
/// and big-endian counter.
type CtrReader<R, C, F> = stream::StreamCipherReader<R, CtrCore<C, F>>;

/// A type alias for a CTR mode stream cipher writer.
///
/// This type represents a writer that encrypts data using CTR mode with a 128-bit block size
/// and big-endian counter.
type CtrWriter<W, C, F> = stream::StreamCipherWriter<W, CtrCore<C, F>>;

/// A type alias for a CTR mode stream cipher reader with 128-bit block size and big-endian counter.
pub(crate) type Ctr128BEReader<R, C> = CtrReader<R, C, Ctr128BE>;

/// A type alias for a CTR mode stream cipher writer with 128-bit block size and big-endian counter.
pub(crate) type Ctr128BEWriter<W, C> = CtrWriter<W, C, Ctr128BE>;

/// A type alias for an AES-256 CBC mode encryption writer.
pub(crate) type EncryptCbcAes256Writer<W> = block::CbcBlockCipherEncryptWriter<W, Aes256, Pkcs7>;

/// A type alias for an AES-256 CBC mode decryption reader.
pub(crate) type DecryptCbcAes256Reader<R> = block::CbcBlockCipherDecryptReader<R, Aes256, Pkcs7>;

/// A type alias for a Camellia-256 CBC mode encryption writer.
pub(crate) type EncryptCbcCamellia256Writer<W> =
    block::CbcBlockCipherEncryptWriter<W, Camellia256, Pkcs7>;

/// A type alias for a Camellia-256 CBC mode decryption reader.
pub(crate) type DecryptCbcCamellia256Reader<R> =
    block::CbcBlockCipherDecryptReader<R, Camellia256, Pkcs7>;

/// An enum representing different encryption writers for PNA archives.
///
/// This enum provides different encryption implementations for writing data to a PNA archive.
/// It supports both block ciphers (AES-256 and Camellia-256 in CBC mode) and stream ciphers
/// (AES-256 and Camellia-256 in CTR mode).
pub(crate) enum CipherWriter<W: Write> {
    /// No encryption, data is written as-is.
    No(W),
    /// AES-256 encryption in CBC mode.
    CbcAes(EncryptCbcAes256Writer<W>),
    /// Camellia-256 encryption in CBC mode.
    CbcCamellia(EncryptCbcCamellia256Writer<W>),
    /// AES-256 encryption in CTR mode.
    CtrAes(Ctr128BEWriter<W, Aes256>),
    /// Camellia-256 encryption in CTR mode.
    CtrCamellia(Ctr128BEWriter<W, Camellia256>),
}

impl<W: Write> Write for CipherWriter<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::No(w) => w.write(buf),
            Self::CbcAes(w) => w.write(buf),
            Self::CbcCamellia(w) => w.write(buf),
            Self::CtrAes(w) => w.write(buf),
            Self::CtrCamellia(w) => w.write(buf),
        }
    }

    #[inline]
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

impl<W: Write> CipherWriter<W> {
    #[inline]
    pub(crate) fn get_mut(&mut self) -> &mut W {
        match self {
            Self::No(w) => w,
            Self::CbcAes(w) => w.get_mut(),
            Self::CbcCamellia(w) => w.get_mut(),
            Self::CtrAes(w) => w.get_mut(),
            Self::CtrCamellia(w) => w.get_mut(),
        }
    }
}

impl<W: Write> TryIntoInner<W> for CipherWriter<W> {
    #[inline]
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

/// An enum representing different decryption readers for PNA archives.
///
/// This enum provides different decryption implementations for reading data from a PNA archive.
/// It supports both block ciphers (AES-256 and Camellia-256 in CBC mode) and stream ciphers
/// (AES-256 and Camellia-256 in CTR mode).
pub(crate) enum DecryptReader<R: Read> {
    /// No decryption, data is read as-is.
    No(R),
    /// AES-256 decryption in CBC mode.
    CbcAes(DecryptCbcAes256Reader<R>),
    /// Camellia-256 decryption in CBC mode.
    CbcCamellia(DecryptCbcCamellia256Reader<R>),
    /// AES-256 decryption in CTR mode.
    CtrAes(Ctr128BEReader<R, Aes256>),
    /// Camellia-256 decryption in CTR mode.
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
    use super::*;
    use cipher::{
        BlockCipherDecrypt, BlockCipherEncrypt, BlockModeDecrypt, BlockModeEncrypt, BlockSizeUser,
        KeyIvInit, KeySizeUser,
    };
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    fn encrypt_cbc<Cipher>(key: &[u8], iv: &[u8], data: &[u8]) -> io::Result<Vec<u8>>
    where
        Cipher: BlockCipherEncrypt,
        cbc::Encryptor<Cipher>: BlockModeEncrypt + KeyIvInit,
    {
        let encryptor = cbc::Encryptor::<Cipher>::new_from_slices(key, iv)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut d = encryptor.encrypt_padded_vec::<Pkcs7>(data);
        let mut e = Vec::from(iv);
        e.append(&mut d);
        Ok(e)
    }

    fn decrypt_cbc<Cipher>(key: &[u8], data: &[u8]) -> io::Result<Vec<u8>>
    where
        Cipher: BlockCipherDecrypt,
        cbc::Decryptor<Cipher>: BlockModeDecrypt + KeyIvInit,
    {
        let decryptor =
            cbc::Decryptor::<Cipher>::new_from_slices(key, &data[0..Cipher::block_size()])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let data = decryptor
            .decrypt_padded_vec::<Pkcs7>(&data[Cipher::block_size()..])
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

    const KAT_KEY: [u8; 32] = [0x11; 32];
    const KAT_IV: [u8; 16] = [0x22; 16];
    const KAT_PLAINTEXT: &[u8; 16] = b"PNA test vector!";

    #[test]
    fn aes256_cbc_matches_openssl_known_answer() {
        // openssl enc -aes-256-cbc -K <0x11 x32> -iv <0x22 x16>
        const EXPECTED: [u8; 32] = [
            0xb4, 0xea, 0x96, 0xc2, 0xfc, 0x15, 0x82, 0x5c, 0xe8, 0x56, 0x90, 0x38, 0x5d, 0x8b,
            0x6c, 0x5f, 0x92, 0xbf, 0x89, 0x6b, 0x07, 0xe1, 0xeb, 0xee, 0xe0, 0xf6, 0x84, 0x38,
            0xae, 0xd6, 0xb6, 0x3e,
        ];
        let ct = encrypt_aes256_cbc(&KAT_KEY, &KAT_IV, KAT_PLAINTEXT).unwrap();
        assert_eq!(&ct[..16], &KAT_IV[..]);
        assert_eq!(&ct[16..], &EXPECTED[..]);
        assert_eq!(
            decrypt_aes256_cbc(&KAT_KEY, &ct).unwrap().as_slice(),
            KAT_PLAINTEXT.as_slice()
        );
    }

    #[test]
    fn camellia256_cbc_matches_openssl_known_answer() {
        // openssl enc -camellia-256-cbc -K <0x11 x32> -iv <0x22 x16>
        const EXPECTED: [u8; 32] = [
            0x47, 0xd8, 0x90, 0x0a, 0xce, 0x45, 0x56, 0xef, 0xf9, 0xff, 0x32, 0xa5, 0xb9, 0x60,
            0x53, 0x29, 0xfe, 0xab, 0xcb, 0x55, 0x93, 0x91, 0x0c, 0xb9, 0xac, 0xfc, 0x2f, 0xcb,
            0x86, 0xc8, 0xa7, 0x8b,
        ];
        let ct = encrypt_camellia256_cbc(&KAT_KEY, &KAT_IV, KAT_PLAINTEXT).unwrap();
        assert_eq!(&ct[..16], &KAT_IV[..]);
        assert_eq!(&ct[16..], &EXPECTED[..]);
        assert_eq!(
            decrypt_camellia256_cbc(&KAT_KEY, &ct).unwrap().as_slice(),
            KAT_PLAINTEXT.as_slice()
        );
    }

    #[test]
    fn aes256_cbc_wrong_key_does_not_recover_plaintext() {
        let ct = encrypt_aes256_cbc(&KAT_KEY, &KAT_IV, KAT_PLAINTEXT).unwrap();
        let wrong_key = [0x99u8; 32];
        if let Ok(recovered) = decrypt_aes256_cbc(&wrong_key, &ct) {
            assert_ne!(recovered.as_slice(), KAT_PLAINTEXT.as_slice())
        }
    }

    #[test]
    fn camellia256_cbc_wrong_key_does_not_recover_plaintext() {
        let ct = encrypt_camellia256_cbc(&KAT_KEY, &KAT_IV, KAT_PLAINTEXT).unwrap();
        let wrong_key = [0x99u8; 32];
        if let Ok(recovered) = decrypt_camellia256_cbc(&wrong_key, &ct) {
            assert_ne!(recovered.as_slice(), KAT_PLAINTEXT.as_slice())
        }
    }
}
