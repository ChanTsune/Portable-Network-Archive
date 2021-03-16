package chunk

import (
	"bytes"
	"hash/crc32"
	"pna/pna"
)

type AHADChunk interface {
	Chunk
	MajorVersion() uint8
	MinorVersion() uint8
	GeneralPurposeBitFlag() uint16
}

type aHADChunk struct {
	majorVersion          uint8
	minorVersion          uint8
	generalPurposeBitFlag uint16
}

func (c *aHADChunk) MajorVersion() uint8 {
	return c.majorVersion
}

func (c *aHADChunk) MinorVersion() uint8 {
	return c.minorVersion
}

func (c *aHADChunk) GeneralPurposeBitFlag() uint16 {
	return c.generalPurposeBitFlag
}

func (c *aHADChunk) CRC() uint32 {
	crc := crc32.NewIEEE()
	crc.Write([]byte(c.Type()))
	crc.Write(c.Data())
	return crc.Sum32()
}

func (c *aHADChunk) Length() uint32 {
	return uint32(len(c.Data()))
}

func (c *aHADChunk) Type() string {
	return "AHAD"
}

func (c *aHADChunk) Data() []byte {
	return bytes.Join([][]byte{
		{c.MajorVersion()},
		{c.MinorVersion()},
		pna.Uint16ToBytes(c.GeneralPurposeBitFlag()),
	}, []byte{})
}

func NewAHADChunk(majorVersion, minorVersion uint8) AHADChunk {
	return &aHADChunk{
		majorVersion:          majorVersion,
		minorVersion:          minorVersion,
		generalPurposeBitFlag: 0,
	}
}
