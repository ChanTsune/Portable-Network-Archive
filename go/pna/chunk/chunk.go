package chunk

type Chunk interface {
	Length() uint32
	Type() string
	Data() []byte
	CRC() uint32
}
