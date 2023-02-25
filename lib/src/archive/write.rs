use crate::{
    archive::{Compression, CompressionLevel, Encryption, HashAlgorithm, Options, PNA_HEADER},
    chunk::{self, ChunkWriter},
    cipher::{encrypt_aes256_cbc, encrypt_camellia256_cbc},
    create_chunk_data_ahed, create_chunk_data_fhed, hash, random,
};
use aes::Aes256;
use camellia::Camellia256;
use cipher::{BlockSizeUser, KeySizeUser};
use flate2::write::DeflateEncoder;
use std::io::{self, Write};
use xz2::write::XzEncoder;
use zstd::stream::write::Encoder as ZstdEncoder;

#[derive(Default)]
pub struct Encoder;

impl Encoder {
    pub fn new() -> Self {
        Self
    }

    pub fn write_header<W: Write>(&self, mut write: W) -> io::Result<ArchiveWriter<W>> {
        write.write_all(PNA_HEADER)?;
        let mut chunk_writer = ChunkWriter::from(write);
        chunk_writer.write_chunk(chunk::AHED, &create_chunk_data_ahed(0, 0, 0))?;
        Ok(ArchiveWriter::new(chunk_writer))
    }
}

pub struct ArchiveWriter<W: Write> {
    w: ChunkWriter<W>,
    // temporary use fields
    options: Options,
    buf: Vec<u8>,
    file_closed: bool,
    // end temporary
    finalized: bool,
}

impl<W: Write> ArchiveWriter<W> {
    fn new(chunk_writer: ChunkWriter<W>) -> Self {
        Self {
            w: chunk_writer,
            options: Options::default(),
            buf: Vec::new(),
            file_closed: true,
            finalized: false,
        }
    }

    pub fn start_file(&mut self, name: &str) -> io::Result<()> {
        self.start_file_with_options(name, Options::default())
    }

    pub fn start_file_with_options(&mut self, name: &str, options: Options) -> io::Result<()> {
        self.end_file()?;
        self.file_closed = false;
        self.options = options;

        self.w.write_chunk(
            chunk::FHED,
            &create_chunk_data_fhed(
                0,
                0,
                self.options.compression as u8,
                self.options.encryption as u8,
                0,
                name,
            ),
        )?;
        Ok(())
    }

    pub fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        self.buf.extend(data);
        Ok(())
    }

    pub fn end_file(&mut self) -> io::Result<()> {
        if self.file_closed {
            return Ok(());
        }
        let mut data = Vec::new();
        {
            let mut writer = io::Cursor::new(&mut data);

            let mut compression_writer: Box<dyn Write> = match self.options.compression {
                Compression::No => Box::new(writer),
                Compression::Deflate => Box::new(DeflateEncoder::new(writer, {
                    if self.options.compression_level == CompressionLevel::default() {
                        flate2::Compression::default()
                    } else {
                        flate2::Compression::new(self.options.compression_level.0 as u32)
                    }
                })),
                Compression::ZStandard => Box::new(
                    ZstdEncoder::new(writer, {
                        if self.options.compression_level == CompressionLevel::default() {
                            0
                        } else {
                            self.options.compression_level.0 as i32
                        }
                    })?
                    .auto_finish(),
                ),
                Compression::XZ => Box::new(XzEncoder::new(writer, {
                    if self.options.compression_level == CompressionLevel::default() {
                        6
                    } else {
                        self.options.compression_level.0 as u32
                    }
                })),
            };

            compression_writer.write_all(&self.buf)?;
            self.buf.clear();
        }
        let data = match self.options.encryption {
            Encryption::No => data,
            encryption @ Encryption::Aes | encryption @ Encryption::Camellia => {
                let salt = random::salt_string();
                let mut password_hash = match self.options.hash_algorithm {
                    HashAlgorithm::Argon2Id => hash::argon2_with_salt(
                        self.options.password.as_ref().unwrap(),
                        Aes256::key_size(),
                        &salt,
                    ),
                    HashAlgorithm::Pbkdf2Sha256 => {
                        hash::pbkdf2_with_salt(self.options.password.as_ref().unwrap(), &salt)
                    }
                };
                let hash = password_hash.hash.take();
                self.w
                    .write_chunk(chunk::PHSF, password_hash.to_string().as_bytes())?;
                if let Encryption::Aes = encryption {
                    let mut iv = vec![0; Aes256::block_size()];
                    random::random_bytes(&mut iv)?;
                    encrypt_aes256_cbc(hash.unwrap().as_bytes(), &iv, &data)?
                } else {
                    let mut iv = vec![0; Camellia256::block_size()];
                    random::random_bytes(&mut iv)?;
                    encrypt_camellia256_cbc(hash.unwrap().as_bytes(), &iv, &data)?
                }
            }
        };

        self.w.write_chunk(chunk::FDAT, &data)?;

        // Write end of file
        self.w.write_chunk(chunk::FEND, &[])?;
        self.file_closed = true;
        Ok(())
    }

    pub fn finalize(&mut self) -> io::Result<()> {
        self.end_file()?;
        if !self.finalized {
            self.w.write_chunk(chunk::AEND, &[])?;
            self.finalized = true;
        }
        Ok(())
    }
}

impl<W: Write> Drop for ArchiveWriter<W> {
    fn drop(&mut self) {
        self.finalize().expect("archive finalize failed.");
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;

    #[test]
    fn encode() {
        let mut file = Vec::new();
        {
            let encoder = Encoder::new();
            let mut writer = encoder.write_header(&mut file).unwrap();
            writer.finalize().unwrap();
        }
        let expected = include_bytes!("../../../resources/empty_archive.pna");
        assert_eq!(file.as_slice(), expected.as_slice());
    }
}
