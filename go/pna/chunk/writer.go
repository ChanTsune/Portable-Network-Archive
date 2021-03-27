package chunk

import (
	"io"
	"pna/pna/constants"
)

type Writer struct {
	writer io.Writer
}

func NewWriter(w io.Writer) *Writer {
	return &Writer{
		writer: w,
	}
}

func (w *Writer) WritePNAHeader() (int, error) {
	return w.writer.Write(constants.Header)
}

func (w *Writer) WriteChunk(chunk Chunk) (int, error) {
	c, err := From(chunk).WriteTo(w.writer)
	return int(c), err
}
