package pna

import (
	"bytes"
	"hash/crc32"
	"io"
)

type Chunk struct {
	Length uint32
	Type   string
	Data   []byte
	CRC    uint32
}

// WriteTo ...
func (c *Chunk) WriteTo(w io.Writer) (int64, error) {
	w.Write(Uint32ToBytes(c.Length))
	w.Write([]byte(c.Type))
	w.Write(c.Data)
	w.Write(Uint32ToBytes(c.CRC))
	return 4 + 4 + int64(c.Length) + 4, nil
}

func (c Chunk) Check() bool {
	crc := crc32.NewIEEE()
	crc.Write([]byte(c.Type))
	crc.Write(c.Data)
	return c.CRC == crc.Sum32()
}

func NewChunk(chunkType string, data []byte) *Chunk {
	crc := crc32.NewIEEE()
	crc.Write([]byte(chunkType))
	crc.Write(data)
	return &Chunk{
		Length: uint32(len(data)),
		Type:   chunkType,
		Data:   data,
		CRC:    crc.Sum32(),
	}
}

func NewAHEDChunk(majorVersion, minorVersion uint8) *Chunk {
	return NewChunk("AHED", bytes.Join([][]byte{
		{majorVersion},
		{minorVersion},
		{0x00, 0x00}, // General purpose bit flag
	}, []byte{}))
}

func NewFHEDChunk(majorVersion, minorVersion, compressionMethod, encryptionMethod, fileType uint8, fileName string) *Chunk {
	return NewChunk("FHED", bytes.Join([][]byte{
		{majorVersion},
		{minorVersion},
		{compressionMethod},
		{encryptionMethod},
		{fileType},
		{0x00}, // Null byte
	}, []byte{}))
}

func NewFDATChunk(data []byte) *Chunk {
	return NewChunk("FDAT", data)
}

func NewFENDChunk() *Chunk {
	return NewChunk("FEND", []byte{})
}

func NewAENDChunk() *Chunk {
	return NewChunk("AEND", []byte{})
}
