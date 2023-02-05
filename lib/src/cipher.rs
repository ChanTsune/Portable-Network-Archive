use aes::Aes256;
use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, BlockSizeUser, KeyIvInit};
use std::io;

pub(crate) fn encrypt_aes256_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> io::Result<Vec<u8>> {
    let encryptor = cbc::Encryptor::<Aes256>::new_from_slices(key, iv)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let mut d = encryptor.encrypt_padded_vec_mut::<Pkcs7>(data);
    let mut e = Vec::from(iv);
    e.append(&mut d);
    Ok(e)
}

pub(crate) fn decrypt_aes256_cbc(key: &[u8], data: &[u8]) -> io::Result<Vec<u8>> {
    let decryptor = cbc::Decryptor::<Aes256>::new_from_slices(key, &data[0..Aes256::block_size()])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let data = decryptor
        .decrypt_padded_vec_mut::<Pkcs7>(&data[Aes256::block_size()..])
        .map_err(|io| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::{decrypt_aes256_cbc, encrypt_aes256_cbc};
    use aes::Aes256;
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
}
