package chunk

import (
	"bytes"
	"encoding/binary"
	"hash/crc32"
	"pna/pna/utils"
)

type AHEDChunk interface {
	Chunk
	MajorVersion() uint8
	MinorVersion() uint8
	GeneralPurposeBitFlag() uint16
}

type aHEDChunk struct {
	majorVersion          uint8
	minorVersion          uint8
	generalPurposeBitFlag uint16
}

func (c *aHEDChunk) MajorVersion() uint8 {
	return c.majorVersion
}

func (c *aHEDChunk) MinorVersion() uint8 {
	return c.minorVersion
}

func (c *aHEDChunk) GeneralPurposeBitFlag() uint16 {
	return c.generalPurposeBitFlag
}

func (c *aHEDChunk) CRC() uint32 {
	crc := crc32.NewIEEE()
	crc.Write([]byte(c.Type()))
	crc.Write(c.Data())
	return crc.Sum32()
}

func (c *aHEDChunk) Length() uint32 {
	return uint32(len(c.Data()))
}

func (c *aHEDChunk) Type() string {
	return "AHED"
}

func (c *aHEDChunk) Data() []byte {
	return bytes.Join([][]byte{
		{c.MajorVersion()},
		{c.MinorVersion()},
		utils.Uint16ToBytes(c.GeneralPurposeBitFlag()),
	}, []byte{})
}

func NewAHEDChunk(majorVersion, minorVersion uint8, generalPurposeBitFlag uint16) AHEDChunk {
	return &aHEDChunk{
		majorVersion:          majorVersion,
		minorVersion:          minorVersion,
		generalPurposeBitFlag: generalPurposeBitFlag,
	}
}

func ToAHEDChunk(c *chunk) AHEDChunk {
	return NewAHEDChunk(c.Data[0], c.Data[1], binary.BigEndian.Uint16(c.Data[2:4]))
}
