package pna

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"io/ioutil"
	"os"
	"path/filepath"
	"pna/pna/chunk"
	"pna/pna/constants"
	"pna/pna/utils"

	"github.com/DataDog/zstd"
)

func ExtractAll(extractTo, name string, password string) error {
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
	reader := chunk.NewReader(file)
	if _, err := reader.ReadPNAHeader(); err != nil {
		return err
	}
	buf := make([]byte, 0)
	var fhad chunk.FHEDChunk
	for {
		cnk, err := reader.ReadChunk()
		if err != nil {
			return err
		}
		switch cnk.Type() {
		case "AHED":
			fmt.Println(cnk)
		case "FHED":
			fhad = chunk.ToFHEDChunk(cnk)
			fmt.Println(cnk)
		case "FDAT":
			buf = append(buf, cnk.Data()...)
		case "FEND":
			extractPath := filepath.Join(extractTo, fhad.FileName())
			if err := os.MkdirAll(filepath.Dir(extractPath), 0755); err != nil {
				return err
			}
			f, err := os.Create(extractPath)
			if err != nil {
				return err
			}
			defer f.Close()
			switch fhad.FileType() {
			case constants.FileTypeNormal:
				switch fhad.EncryptionMethod() {
				case constants.AesEncryption:
					if len(password) == 0 {
						return errors.New("this file is encrypted but password not given")
					}
					buf, err = utils.AESDecryption(buf, password)
					if err != nil {
						return err
					}
				case constants.CamelliaEncryption:
					if len(password) == 0 {
						return errors.New("this file is encrypted but password not given")
					}
					buf, err = utils.CamelliaDecryption(buf, password)
					if err != nil {
						return err
					}
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
			case constants.FileTypeDirectory:
				if err := os.MkdirAll(fhad.FileName(), 0755); err != nil {
					return nil
				}
			}
		case "AEND":
			return nil
		}
	}
}

func Archive(name string, names []string, options ...Option) error {
	option := mergeOption(options...)
	if err := option.validate(); err != nil {
		return err
	}
	wf, err := os.Create(name)
	if err != nil {
		return err
	}
	defer wf.Close()
	pnaWriter, err := NewWriter(wf)
	if err != nil {
		return err
	}

	for _, fileName := range names {
		fileWriter, err := pnaWriter.CreateWithFileInfo(NewFileInfo(
			option.compressionMethod,
			option.encryptionMethod,
			constants.FileTypeNormal,
			fileName,
			option.password,
		))
		if err != nil {
			return err
		}
		data, err := ioutil.ReadFile(fileName)
		if err != nil {
			fmt.Print(err)
			return err
		}
		fileWriter.Write(data)
	}
	return pnaWriter.Close()
}

func ArchiveAll(dir, name string, options ...Option) error {
	option := mergeOption(options...)
	if err := option.validate(); err != nil {
		return err
	}
	wf, err := os.Create(name)
	if err != nil {
		return err
	}
	defer wf.Close()
	pnaWriter, err := NewWriter(wf)
	if err != nil {
		return err
	}

	filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		fmt.Println(path)
		if info.IsDir() {
			return nil
		}
		fileWriter, err := pnaWriter.CreateWithFileInfo(NewFileInfo(
			option.compressionMethod,
			option.encryptionMethod,
			constants.FileTypeNormal,
			path,
			option.password,
		))
		if err != nil {
			return err
		}
		data, err := ioutil.ReadFile(path)
		if err != nil {
			fmt.Print(err)
			return err
		}
		fileWriter.Write(data)

		return nil
	})
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
