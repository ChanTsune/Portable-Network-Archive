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
	raw    *[]byte
	length uint32
	type_  string
	data   []byte
	crc    uint32
}

func (c *chunk) Length() uint32 {
	return c.length
}

func (c *chunk) Type() string {
	return c.type_
}

func (c *chunk) Data() []byte {
	return c.data
}

func (c *chunk) CRC() uint32 {
	return c.crc
}

func From(c Chunk) *chunk {
	return &chunk{
		length: c.Length(),
		type_:  c.Type(),
		data:   c.Data(),
		crc:    c.CRC(),
	}
}

// WriteTo ...
func (c *chunk) WriteTo(w io.Writer) (int64, error) {
	w.Write(utils.Uint32ToBytes(c.Length()))
	w.Write([]byte(c.Type()))
	w.Write(c.Data())
	w.Write(utils.Uint32ToBytes(c.CRC()))
	return 4 + 4 + int64(c.Length()) + 4, nil
}

func (c chunk) Check() bool {
	crc := crc32.NewIEEE()
	crc.Write([]byte(c.Type()))
	crc.Write(c.Data())
	return c.CRC() == crc.Sum32()
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
		length: length,
		type_:  string(type_),
		data:   data,
		crc:    binary.BigEndian.Uint32(crc32_),
	}, nil
}

func NewChunk(chunkType string, data []byte) *chunk {
	crc := crc32.NewIEEE()
	crc.Write([]byte(chunkType))
	crc.Write(data)
	return &chunk{
		length: uint32(len(data)),
		type_:  chunkType,
		data:   data,
		crc:    crc.Sum32(),
	}
}
