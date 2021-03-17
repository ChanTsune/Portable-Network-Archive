package pna

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"pna/pna/chunk"
	"pna/pna/constants"
)

type PnaFile struct {
}

func Open(path string) (*PnaFile, error) {
	return &PnaFile{}, nil
}

func (f *PnaFile) Close() error {
	return nil
}

func ExtractAll(extractTo, name string) error {
	isPna, err := IsPnaFile(name)
	if err != nil {
		return err
	}
	if !isPna {
		return fmt.Errorf("%s is not pna file.", name)
	}
	file, err := os.Open(name)
	defer file.Close()
	if err != nil {
		return err
	}
	if _, err := chunk.ReadHeader(file); err != nil {
		return err
	}
	buf := make([]byte, 0)
	var fhad chunk.FHEDChunk
	for {
		cnk, err := chunk.ReadChunk(file)
		if err != nil {
			return err
		}
		switch cnk.Type {
		case "AHED":
			fmt.Println(cnk)
		case "FHED":
			fhad = chunk.ToFHEDChunk(cnk)
			fmt.Println(cnk)
		case "FDAT":
			buf = append(buf, cnk.Data...)
		case "FEND":
			switch fhad.EncryptionMethod() {
			case constants.NoCompression:
				extractPath := filepath.Join(extractTo, fhad.FileName())
				os.MkdirAll(filepath.Dir(extractPath), 0755)
				f, err := os.Create(extractPath)
				defer f.Close()
				if err != nil {
					return err
				}
				f.Write(buf)
			}
			buf = make([]byte, 0)
			fmt.Println(cnk)
		case "AEND":
			return nil
		}
	}
}

func IsPnaFile(name string) (bool, error) {
	file, err := os.Open(name)
	defer file.Close()
	if err != nil {
		return false, err
	}
	headerLen := len(constants.Header)
	header := make([]byte, headerLen)
	fh, err := file.Read(header)
	if err == io.EOF || fh != headerLen || bytes.Compare(header, constants.Header) != 0 {
		return false, nil
	}
	return true, nil
}

func IsPna(data []byte) bool {
	l := len(constants.Header)
	if len(data) < l {
		return false
	}
	return bytes.Compare(data[:l], constants.Header) == 0
}
