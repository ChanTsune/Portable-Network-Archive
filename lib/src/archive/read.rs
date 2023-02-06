use crate::archive::{item::Item, Compression, Encryption, PNA_HEADER};
use crate::chunk::{self, from_chunk_data_fhed, ChunkReader};
use crate::cipher::{decrypt_aes256_cbc, decrypt_camellia256_cbc};
use crate::hash::verify_password;
use std::io::{self, Cursor, Read, Seek};

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
    pub fn read(&mut self, password: Option<&str>) -> io::Result<Option<Item>> {
        let mut all_data: Vec<u8> = vec![];
        let mut info = None;
        let mut phsf = None;
        loop {
            let (chunk_type, mut raw_data) = self.r.read_chunk()?;
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
                chunk::AEND => return Ok(None),
                chunk::FDAT => all_data.append(&mut raw_data),
                _ => continue,
            }
        }
        let info = info.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                String::from("FHED chunk not found"),
            )
        })?;
        if info.major != 0 || info.minor != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "item version {}.{} is not supported.",
                    info.major, info.minor
                ),
            ));
        }
        let all_data = match info.encryption {
            Encryption::No => all_data,
            Encryption::Aes | Encryption::Camellia => {
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
                if let Encryption::Aes = info.encryption {
                    decrypt_aes256_cbc(phsf.hash.unwrap().as_bytes(), &all_data)?
                } else {
                    decrypt_camellia256_cbc(phsf.hash.unwrap().as_bytes(), &all_data)?
                }
            }
        };
        let reader: Box<dyn Read> = match info.compression {
            Compression::No => Box::new(Cursor::new(all_data)),
            Compression::Deflate => {
                Box::new(flate2::read::DeflateDecoder::new(Cursor::new(all_data)))
            }
            Compression::ZStandard => Box::new(zstd::Decoder::new(Cursor::new(all_data))?),
            Compression::XZ => Box::new(xz2::read::XzDecoder::new(Cursor::new(all_data))),
        };
        Ok(Some(Item { info, reader }))
    }
}

#[cfg(test)]
mod tests {
    use super::Decoder;
    use std::io::Cursor;

    #[test]
    fn decode() {
        let file_bytes = include_bytes!("../../../resources/empty_archive.pna");
        let reader = Cursor::new(file_bytes);
        let decoder = Decoder::new();
        let mut reader = decoder.read_header(reader).unwrap();
        reader.read(None).unwrap();
    }
}
