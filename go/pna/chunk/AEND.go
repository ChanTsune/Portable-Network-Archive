package chunk

var AENDChunk = NewChunk("AEND", []byte{})

func NewAENDChunk() *chunk {
	return AENDChunk
}
