use std::io::{self, Write};

use crate::{ChunkType, Crc32, PNA_HEADRE};

#[derive(Default)]
pub struct Encoder;

impl<W> From<W> for ChunkWriter<W>
where
    W: Write,
{
    fn from(writer: W) -> Self {
        Self { w: writer }
    }
}

impl Encoder {
    pub fn new() -> Self {
        Self
    }

    pub fn write_header<W: Write>(&self, mut write: W) -> io::Result<ChunkWriter<W>> {
        write.write_all(PNA_HEADRE)?;
        Ok(ChunkWriter { w: write })
    }
}

pub struct ChunkWriter<W> {
    w: W,
}

impl<W: Write> ChunkWriter<W> {
    pub fn write_chunk(&mut self, type_: ChunkType, data: &[u8]) -> io::Result<()> {
        let mut crc = Crc32::new();
        // write length
        let length = data.len() as u32;
        self.w.write_all(&length.to_be_bytes())?;

        // write chunk type
        self.w.write_all(&type_.0)?;
        crc.update(&type_.0);

        // write data
        self.w.write_all(data)?;
        crc.update(data);

        // write crc32
        self.w.write_all(&crc.finalize().to_be_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufWriter;

    use super::Encoder;
    use crate::{chunk, create_chunk_data_ahed};

    #[test]
    fn encode() {
        let file = tempfile::tempfile().unwrap();
        let encoder = Encoder::new();
        let mut writer = encoder.write_header(BufWriter::new(file)).unwrap();
        writer
            .write_chunk(chunk::AHED, &create_chunk_data_ahed(0, 0, 0))
            .unwrap();
        writer.write_chunk(chunk::AEND, &[]).unwrap();
    }
}
