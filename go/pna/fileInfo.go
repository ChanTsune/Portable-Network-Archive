package pna

import (
	"pna/pna/chunk"
	"pna/pna/constants"
)

type FileInfo struct {
	compressionMethod constants.CompressionMethod
	encryptionMethod  constants.EncryptionMethod
	fileType          constants.FileType
	fileName          string
	password          string
}

func NewFileInfo(compressionMethod constants.CompressionMethod, encryptionMethod constants.EncryptionMethod, fileType constants.FileType, fileName, password string) *FileInfo {
	return &FileInfo{
		compressionMethod: compressionMethod,
		encryptionMethod:  encryptionMethod,
		fileType:          fileType,
		fileName:          fileName,
		password:          password,
	}
}

func (f *FileInfo) CompressionMethod() constants.CompressionMethod {
	return f.compressionMethod
}

func (f *FileInfo) EncryptionMethod() constants.EncryptionMethod {
	return f.encryptionMethod
}

func (f *FileInfo) FileType() constants.FileType {
	return f.fileType
}

func (f *FileInfo) FileName() string {
	return f.fileName
}

func (f *FileInfo) ToFHEDChunk() chunk.FHEDChunk {
	return chunk.NewFHEDChunk(
		constants.MajorVersion,
		constants.MinorVersion,
		f.compressionMethod,
		f.encryptionMethod,
		f.fileType,
		f.fileName,
	)
}
