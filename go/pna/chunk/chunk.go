package chunk

import (
	"encoding/binary"
	"hash/crc32"
	"io"
	"pna/pna/constants"
	"pna/pna/utils"
)

type Chunk interface {
	Length() uint32
	Type() string
	Data() []byte
	CRC() uint32
}

type chunk struct {
	Length uint32
	Type   string
	Data   []byte
	CRC    uint32
}

func From(c Chunk) *chunk {
	return &chunk{
		Length: c.Length(),
		Type:   c.Type(),
		Data:   c.Data(),
		CRC:    c.CRC(),
	}
}

// WriteTo ...
func (c *chunk) WriteTo(w io.Writer) (int64, error) {
	w.Write(utils.Uint32ToBytes(c.Length))
	w.Write([]byte(c.Type))
	w.Write(c.Data)
	w.Write(utils.Uint32ToBytes(c.CRC))
	return 4 + 4 + int64(c.Length) + 4, nil
}

func (c chunk) Check() bool {
	crc := crc32.NewIEEE()
	crc.Write([]byte(c.Type))
	crc.Write(c.Data)
	return c.CRC == crc.Sum32()
}

func ReadHeader(r io.Reader) ([]byte, error) {
	h := make([]byte, len(constants.Header))
	if _, err := r.Read(h); err != nil {
		return nil, err
	}
	return h, nil
}

func ReadChunk(r io.Reader) (*chunk, error) {
	length_ := make([]byte, 4)
	if _, err := r.Read(length_); err != nil {
		return nil, err
	}
	length := binary.BigEndian.Uint32(length_)
	type_ := make([]byte, 4)
	if _, err := r.Read(type_); err != nil {
		return nil, err
	}
	data := make([]byte, length)
	if _, err := r.Read(data); err != nil {
		return nil, err
	}
	crc32_ := make([]byte, 4)
	if _, err := r.Read(crc32_); err != nil {
		return nil, err
	}
	return &chunk{
		Length: length,
		Type:   string(type_),
		Data:   data,
		CRC:    binary.BigEndian.Uint32(crc32_),
	}, nil
}

func NewChunk(chunkType string, data []byte) *chunk {
	crc := crc32.NewIEEE()
	crc.Write([]byte(chunkType))
	crc.Write(data)
	return &chunk{
		Length: uint32(len(data)),
		Type:   chunkType,
		Data:   data,
		CRC:    crc.Sum32(),
	}
}
