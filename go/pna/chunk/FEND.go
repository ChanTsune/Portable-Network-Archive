package chunk

var FENDChunk = NewChunk("FEND", []byte{})

func NewFENDChunk() *chunk {
	return FENDChunk
}
