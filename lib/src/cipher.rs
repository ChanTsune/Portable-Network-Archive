mod block;

use aes::Aes256;
use camellia::Camellia256;
use cipher::{block_padding::Pkcs7, BlockCipher, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use std::io;

pub(crate) type EncryptCbcAes256Writer<W> = block::CbcBlockCipherEncryptWriter<W, Aes256, Pkcs7>;
pub(crate) type DecryptCbcAes256Reader<R> = block::CbcBlockCipherDecryptReader<R, Aes256, Pkcs7>;
pub(crate) type EncryptCbcCamellia256Writer<W> =
    block::CbcBlockCipherEncryptWriter<W, Camellia256, Pkcs7>;
pub(crate) type DecryptCbcCamellia256Reader<R> =
    block::CbcBlockCipherDecryptReader<R, Camellia256, Pkcs7>;

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

fn decrypt_cbc<Cipher: BlockDecryptMut + BlockCipher>(
    key: &[u8],
    data: &[u8],
) -> io::Result<Vec<u8>>
where
    Cipher: BlockDecryptMut + BlockCipher,
    cbc::Decryptor<Cipher>: KeyIvInit,
{
    let decryptor = cbc::Decryptor::<Cipher>::new_from_slices(key, &data[0..Cipher::block_size()])
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

pub(crate) fn encrypt_camellia256_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> io::Result<Vec<u8>> {
    encrypt_cbc::<Camellia256>(key, iv, data)
}

pub(crate) fn decrypt_camellia256_cbc(key: &[u8], data: &[u8]) -> io::Result<Vec<u8>> {
    decrypt_cbc::<Camellia256>(key, data)
}

#[cfg(test)]
mod tests {
    use super::{
        decrypt_aes256_cbc, decrypt_camellia256_cbc, encrypt_aes256_cbc, encrypt_camellia256_cbc,
    };
    use aes::Aes256;
    use camellia::Camellia256;
    use cipher::{BlockSizeUser, KeySizeUser};

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
