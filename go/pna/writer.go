package pna

import (
	"bufio"
	"errors"
	"io"
	"pna/pna/chunk"
	"pna/pna/constants"
	"pna/pna/utils"

	"github.com/DataDog/zstd"
)

type Writer struct {
	w      *bufio.Writer
	files  []*file
	closed bool
}

type bufWriter struct {
	buf []byte
}

func newBufWriter() *bufWriter {
	return &bufWriter{
		buf: make([]byte, 0, 2048),
	}
}

func (b *bufWriter) Write(p []byte) (n int, err error) {
	b.buf = append(b.buf, p...)
	return len(p), nil
}

type file struct {
	bufWriter *bufWriter
	fileInfo  *FileInfo
}

func newFile(bufWriter *bufWriter, fileInfo *FileInfo) *file {
	return &file{
		bufWriter: bufWriter,
		fileInfo:  fileInfo,
	}
}

func NewWriter(w io.Writer) (*Writer, error) {
	return &Writer{
		w:      bufio.NewWriter(w),
		files:  make([]*file, 0, 1024),
		closed: false,
	}, nil
}

func (w *Writer) Create(name string) (io.Writer, error) {
	return w.CreateWithFileInfo(NewFileInfo(
		constants.ZstdCompression,
		constants.NoEncryption,
		constants.FileTypeNormal,
		name,
		"", // Empty string to no encryption
	))
}

func (w *Writer) CreateWithFileInfo(f *FileInfo) (io.Writer, error) {
	buf := newBufWriter()
	w.files = append(w.files, newFile(buf, f))
	return buf, nil
}

func (w *Writer) Close() error {
	var err error
	if w.closed {
		return errors.New("pna: writer closed twice")
	}
	w.closed = true
	chunkWriter := chunk.NewWriter(w.w)
	chunkWriter.WritePNAHeader()
	for _, file := range w.files {
		option := file.fileInfo
		chunkWriter.WriteChunk(option.ToFHEDChunk())
		data := file.bufWriter.buf
		switch option.compressionMethod {
		case constants.NoCompression:
		case constants.ZstdCompression:
			data, err = zstd.Compress(nil, data)
			if err != nil {
				return err
			}
		case constants.LzmaCompression:
			panic("unsupported lzma compress")
		case constants.DeflateCompression:
			panic("unsupported deflate compress")
		}
		switch option.encryptionMethod {
		case constants.AesEncryption:
			data, err = utils.AesEncryption(data, option.password)
			if err != nil {
				return err
			}
		case constants.CamelliaEncryption:
			data, err = utils.CamelliaEncryption(data, option.password)
			if err != nil {
				return err
			}
		case constants.NoEncryption:
		}
		chunkWriter.WriteChunk(chunk.NewFDATChunk(data))
		chunkWriter.WriteChunk(chunk.NewFENDChunk())
	}
	chunkWriter.WriteChunk(chunk.NewAENDChunk())
	return w.w.Flush()
}
