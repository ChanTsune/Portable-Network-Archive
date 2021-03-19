package pna

import (
	"bytes"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"os"
	"path/filepath"
	"pna/pna/chunk"
	"pna/pna/constants"

	"github.com/DataDog/zstd"
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
			extractPath := filepath.Join(extractTo, fhad.FileName())
			os.MkdirAll(filepath.Dir(extractPath), 0755)
			f, err := os.Create(extractPath)
			defer f.Close()
			if err != nil {
				return err
			}
			switch fhad.CompressionMethod() {
			case constants.NoCompression:
				f.Write(buf)
			case constants.ZstdCompression:
				dst, err := zstd.Decompress(nil, buf)
				if err != nil {
					return err
				}
				f.Write(dst)
			}
			buf = make([]byte, 0)
			fmt.Println(cnk)
		case "AEND":
			return nil
		}
	}
}

func ArchiveAll(dir, name string) error {
	wf, err := os.Create(name)
	defer wf.Close()
	if err != nil {
		log.Fatalln(err)
	}
	wf.Write(constants.Header)
	chunk.From(chunk.NewAHEDChunk(
		constants.MajorVersion,
		constants.MinorVersion,
		0,
	)).WriteTo(wf)
	filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		fmt.Println(path)
		if info.IsDir() {
			return nil
		}
		fhed := chunk.From(chunk.NewFHEDChunk(
			constants.MajorVersion,
			constants.MinorVersion,
			constants.ZstdCompression,
			constants.NoEncryption,
			constants.FileTypeNormal,
			path,
		))
		data, err := ioutil.ReadFile(path)
		if err != nil {
			fmt.Print(err)
			return err
		}
		cData, err := zstd.Compress(nil, data)
		if err != nil {
			return err
		}
		fdat := chunk.NewFDATChunk(cData)
		fhed.WriteTo(wf)
		fdat.WriteTo(wf)
		chunk.NewFENDChunk().WriteTo(wf)

		return nil
	})

	chunk.NewAENDChunk().WriteTo(wf)
	return nil
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
