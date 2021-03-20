package chunk

func NewFDATChunk(data []byte) *chunk {
	return NewChunk("FDAT", data)
}
