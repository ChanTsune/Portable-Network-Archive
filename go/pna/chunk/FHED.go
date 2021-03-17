package chunk

import (
	"bytes"
	"hash/crc32"
)

type FHEDChunk interface {
	Chunk
	MajorVersion() uint8
	MinorVersion() uint8
	CompressionMethod() uint8
	EncryptionMethod() uint8
	FileType() uint8
	FileName() string
}

type fHEDChunk struct {
	majorVersion      uint8
	minorVersion      uint8
	compressionMethod uint8
	encryptionMethod  uint8
	fileType          uint8
	fileName          string
}

func (c *fHEDChunk) Length() uint32 {
	return uint32(len(c.Data()))
}
func (c *fHEDChunk) Type() string {
	return "FHED"
}
func (c *fHEDChunk) Data() []byte {
	return bytes.Join([][]byte{
		{c.majorVersion},
		{c.minorVersion},
		{c.compressionMethod},
		{c.encryptionMethod},
		{c.fileType},
		{0x00}, // Null byte
		[]byte(c.fileName),
	}, []byte{})
}
func (c *fHEDChunk) CRC() uint32 {
	crc := crc32.NewIEEE()
	crc.Write([]byte(c.Type()))
	crc.Write(c.Data())
	return crc.Sum32()
}

func (c *fHEDChunk) MajorVersion() uint8 {
	return c.majorVersion
}
func (c *fHEDChunk) MinorVersion() uint8 {
	return c.minorVersion
}
func (c *fHEDChunk) CompressionMethod() uint8 {
	return c.compressionMethod
}
func (c *fHEDChunk) EncryptionMethod() uint8 {
	return c.encryptionMethod
}
func (c *fHEDChunk) FileType() uint8 {
	return c.fileType
}
func (c *fHEDChunk) FileName() string {
	return c.fileName
}

func NewFHEDChunk(majorVersion, minorVersion, compressionMethod, encryptionMethod, fileType uint8, fileName string) FHEDChunk {
	return &fHEDChunk{
		majorVersion:      majorVersion,
		minorVersion:      minorVersion,
		compressionMethod: compressionMethod,
		encryptionMethod:  encryptionMethod,
		fileType:          fileType,
		fileName:          fileName,
	}
}

func ToFHEDChunk(c *chunk) FHEDChunk {
	return NewFHEDChunk(
		c.Data[0],
		c.Data[1],
		c.Data[2],
		c.Data[3],
		c.Data[4],
		string(c.Data[6:]),
	)
}
