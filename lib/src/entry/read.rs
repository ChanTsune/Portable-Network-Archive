use crate::{
    CipherMode, Compression, Encryption,
    cipher::{Ctr128BEReader, DecryptCbcAes256Reader, DecryptCbcCamellia256Reader, DecryptReader},
    compress::DecompressReader,
    hash::derive_password_hash,
};
use aes::Aes256;
use camellia::Camellia256;
use crypto_common::BlockSizeUser;
use std::io::{self, Read};

/// Decrypt reader according to an encryption type.
pub(crate) fn decrypt_reader<R: Read>(
    mut reader: R,
    encryption: Encryption,
    cipher_mode: CipherMode,
    phsf: Option<&str>,
    password: Option<&[u8]>,
) -> io::Result<DecryptReader<R>> {
    Ok(match encryption {
        Encryption::No => DecryptReader::No(reader),
        encryption @ (Encryption::Aes | Encryption::Camellia) => {
            let s = phsf.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "`PHSF` chunk not found")
            })?;
            let phsf = derive_password_hash(
                s,
                password.ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Password was not provided")
                })?,
            )?;
            let hash = phsf
                .hash
                .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "Failed to get hash"))?;
            let key = hash.as_bytes();
            match (encryption, cipher_mode) {
                (Encryption::Aes, CipherMode::CBC) => {
                    let mut iv = vec![0; Aes256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CbcAes(DecryptCbcAes256Reader::new(reader, key, &iv)?)
                }
                (Encryption::Aes, CipherMode::CTR) => {
                    let mut iv = vec![0u8; Aes256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CtrAes(Ctr128BEReader::new(reader, key, &iv)?)
                }
                (Encryption::Camellia, CipherMode::CBC) => {
                    let mut iv = vec![0; Camellia256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CbcCamellia(DecryptCbcCamellia256Reader::new(reader, key, &iv)?)
                }
                _ => {
                    let mut iv = vec![0u8; Camellia256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CtrCamellia(Ctr128BEReader::new(reader, key, &iv)?)
                }
            }
        }
    })
}

/// Decompress reader according to a compression type.
pub(crate) fn decompress_reader<R: Read>(
    reader: R,
    compression: Compression,
) -> io::Result<DecompressReader<R>> {
    Ok(match compression {
        Compression::No => DecompressReader::No(reader),
        Compression::Deflate => DecompressReader::Deflate(flate2::read::ZlibDecoder::new(reader)),
        Compression::ZStandard => DecompressReader::ZStd(zstd::Decoder::new(reader)?),
        Compression::XZ => DecompressReader::Xz(liblzma::read::XzDecoder::new(reader)),
    })
}

pub(crate) struct EntryReader<R: Read>(pub(crate) DecompressReader<DecryptReader<R>>);

impl<R: Read> Read for EntryReader<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}
