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
	w.Write(uint32ToBytes(c.Length))
	w.Write([]byte(c.Type))
	w.Write(c.Data)
	w.Write(uint32ToBytes(c.CRC))
	return 4 + 4 + int64(c.Length) + 4, nil
}

func NewChunk(chunkType string, data []byte) *Chunk {
	return &Chunk{
		Length: uint32(len(data)),
		Type:   chunkType,
		Data:   data,
		CRC:    crc32.ChecksumIEEE(data),
	}
}

func NewAHEDChunk() *Chunk {
	return NewChunk("AHED", bytes.Join([][]byte{}, []byte{}))
}

func NewFHEDChunk() *Chunk {
	return NewChunk("FHED", []byte{})
}

func NewFDATChunk() *Chunk {
	return NewChunk("FDAT", []byte{})
}

func NewFENDChunk() *Chunk {
	return NewChunk("FEND", []byte{})
}

func NewAENDChunk() *Chunk {
	return NewChunk("AEND", []byte{})
}
