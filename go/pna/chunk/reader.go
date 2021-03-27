package chunk

import (
	"io"
	"pna/pna/constants"
)

type Reader struct {
	reader io.Reader
}

func NewReader(r io.Reader) *Reader {
	return &Reader{
		reader: r,
	}
}

func (r *Reader) ReadPNAHeader() ([]byte, error) {
	b := make([]byte, len(constants.Header))
	_, err := r.reader.Read(b)
	return b, err
}

func (r *Reader) ReadChunk() (*chunk, error) {
	return ReadChunk(r.reader)
}
