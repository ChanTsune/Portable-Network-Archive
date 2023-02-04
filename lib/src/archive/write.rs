use crate::archive::item::{Compression, Encryption, Options};
use crate::cipher::encrypt_aes256;
use crate::{
    archive::PNA_HEADER,
    chunk::{self, ChunkWriter},
    create_chunk_data_ahed, create_chunk_data_fhed, hash, random,
};
use aes::cipher::KeySizeUser;
use aes::Aes256;
use cbc::cipher::BlockSizeUser;
use std::io::{self, Write};

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
        std::mem::swap(&mut data, &mut self.buf);

        let data = match self.options.compression {
            Compression::No => data,
            Compression::Deflate => {
                let mut dist = Vec::new();
                let mut encoder = flate2::read::DeflateEncoder::new(
                    data.as_slice(),
                    flate2::Compression::default(),
                );
                io::copy(&mut encoder, &mut dist)?;
                dist
            }
            Compression::ZStandard => zstd::encode_all(data.as_slice(), 0)?,
            Compression::XZ => {
                let mut dist = Vec::new();
                let mut reader = xz2::read::XzEncoder::new(data.as_slice(), 6);
                io::copy(&mut reader, &mut dist)?;
                dist
            }
        };

        let data = match self.options.encryption {
            Encryption::No => data,
            Encryption::Aes => {
                let salt = random::salt_string();
                let mut password_hash = hash::argon2_with_salt(
                    self.options.password.as_ref().unwrap(),
                    Aes256::key_size(),
                    &salt,
                );
                let hash = password_hash.hash.take();
                self.w
                    .write_chunk(chunk::PHSF, password_hash.to_string().as_bytes())?;

                let mut iv = vec![0; Aes256::block_size()];
                random::random_bytes(&mut iv)?;
                encrypt_aes256(hash.unwrap().as_bytes(), &iv, &data)?
            }
            Encryption::Camellia => todo!("Camellia encryption"),
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
        let file = tempfile::tempfile().unwrap();
        let encoder = Encoder::new();
        let mut writer = encoder.write_header(file).unwrap();
        writer.finalize().unwrap()
    }
}
