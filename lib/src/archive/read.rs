use crate::{
    archive::{
        item::{Entry, RawEntry},
        CipherMode, Compression, Encryption, PNA_HEADER,
    },
    chunk::{self, from_chunk_data_fhed, ChunkReader},
    cipher::{Ctr128BEReader, DecryptCbcAes256Reader, DecryptCbcCamellia256Reader},
    hash::verify_password,
};
use aes::Aes256;
use camellia::Camellia256;
use cipher::BlockSizeUser;
use std::io::{self, Cursor, Read, Seek};
use std::sync::Mutex;

#[derive(Default)]
pub struct Decoder;

impl Decoder {
    pub fn new() -> Self {
        Self
    }

    pub fn read_header<R: Read + Seek>(&self, mut reader: R) -> io::Result<ArchiveReader<R>> {
        let mut header = [0u8; PNA_HEADER.len()];
        reader.read_exact(&mut header)?;
        if &header != PNA_HEADER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                String::from("not pna format"),
            ));
        }
        let mut chunk_reader = ChunkReader::from(reader);
        // Read `AHED` chunk
        let (_, _) = chunk_reader.read_chunk()?;
        Ok(ArchiveReader { r: chunk_reader })
    }
}

pub struct ArchiveReader<R> {
    r: ChunkReader<R>,
}

impl<R: Read> ArchiveReader<R> {
    /// Read the next chunks from `FHED` to `FEND`
    fn next_raw_item(&mut self) -> io::Result<Option<RawEntry>> {
        let mut chunks = Vec::new();
        loop {
            let (chunk_type, raw_data) = self.r.read_chunk()?;
            match chunk_type {
                chunk::FEND => break,
                chunk::AEND => return Ok(None),
                _ => chunks.push((chunk_type, raw_data)),
            }
        }
        Ok(Some(RawEntry { chunks }))
    }

    pub fn read(&mut self, password: Option<&str>) -> io::Result<Option<Entry>> {
        let raw_entry = self.next_raw_item()?;
        let raw_entry = if let Some(raw_entry) = raw_entry {
            raw_entry
        } else {
            return Ok(None);
        };
        let mut all_data: Vec<u8> = vec![];
        let mut info = None;
        let mut phsf = None;
        for (chunk_type, mut raw_data) in raw_entry.chunks {
            match chunk_type {
                chunk::FEND => break,
                chunk::FHED => {
                    info = Some(from_chunk_data_fhed(&raw_data)?);
                }
                chunk::PHSF => {
                    phsf = Some(
                        String::from_utf8(raw_data)
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    );
                }
                chunk::FDAT => all_data.append(&mut raw_data),
                _ => continue,
            }
        }
        let header = info.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                String::from("FHED chunk not found"),
            )
        })?;
        if header.major != 0 || header.minor != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "item version {}.{} is not supported.",
                    header.major, header.minor
                ),
            ));
        }
        let raw_data_reader = Cursor::new(all_data);
        let decrypt_reader: Box<dyn Read + Sync + Send> = match header.encryption {
            Encryption::No => Box::new(raw_data_reader),
            encryption @ Encryption::Aes | encryption @ Encryption::Camellia => {
                let s = phsf.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        String::from("Item is encrypted, but `PHSF` chunk not found"),
                    )
                })?;
                let phsf = verify_password(
                    &s,
                    password.ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            String::from("Item is encrypted, but password was not provided"),
                        )
                    })?,
                );
                let hash = phsf.hash.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::Unsupported,
                        String::from("Failed to get hash"),
                    )
                })?;
                match (encryption, header.cipher_mode) {
                    (Encryption::Aes, CipherMode::CBC) => Box::new(DecryptCbcAes256Reader::new(
                        raw_data_reader,
                        hash.as_bytes(),
                    )?),
                    (Encryption::Aes, CipherMode::CTR) => {
                        Box::new(aes_ctr_cipher_reader(raw_data_reader, hash.as_bytes())?)
                    }
                    (Encryption::Camellia, CipherMode::CBC) => Box::new(
                        DecryptCbcCamellia256Reader::new(raw_data_reader, hash.as_bytes())?,
                    ),
                    _ => Box::new(camellia_ctr_cipher_reader(
                        raw_data_reader,
                        hash.as_bytes(),
                    )?),
                }
            }
        };
        let reader: Box<dyn Read + Sync + Send> = match header.compression {
            Compression::No => decrypt_reader,
            Compression::Deflate => Box::new(flate2::read::DeflateDecoder::new(decrypt_reader)),
            Compression::ZStandard => Box::new(MutexRead::new(zstd::Decoder::new(decrypt_reader)?)),
            Compression::XZ => Box::new(xz2::read::XzDecoder::new(decrypt_reader)),
        };
        Ok(Some(Entry { header, reader }))
    }
}

// NOTE: zstd crate not support Sync + Send trait
struct MutexRead<R: Read>(Mutex<R>);

impl<R: Read> MutexRead<R> {
    fn new(reader: R) -> Self {
        Self(Mutex::new(reader))
    }
}

impl<R: Read> Read for MutexRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let reader = self.0.get_mut().unwrap();
        reader.read(buf)
    }
}

fn aes_ctr_cipher_reader<R: Read>(
    mut reader: R,
    key: &[u8],
) -> io::Result<Ctr128BEReader<R, Aes256>> {
    let mut iv = vec![0u8; Aes256::block_size()];
    reader.read_exact(&mut iv)?;
    Ctr128BEReader::new(reader, key, &iv)
}

fn camellia_ctr_cipher_reader<R: Read>(
    mut reader: R,
    key: &[u8],
) -> io::Result<Ctr128BEReader<R, Camellia256>> {
    let mut iv = vec![0u8; Camellia256::block_size()];
    reader.read_exact(&mut iv)?;
    Ctr128BEReader::new(reader, key, &iv)
}

#[cfg(test)]
mod tests {
    use super::Decoder;
    use std::io::Cursor;

    #[test]
    fn decode() {
        let file_bytes = include_bytes!("../../../resources/test/empty.pna");
        let reader = Cursor::new(file_bytes);
        let decoder = Decoder::new();
        let mut reader = decoder.read_header(reader).unwrap();
        reader.read(None).unwrap();
    }
}
