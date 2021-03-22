package pna

import (
	"errors"
	"pna/pna/constants"
)

type pnaArchiveConfig struct {
	password          string
	encryptionMethod  constants.EncryptionMethod
	compressionMethod constants.CompressionMethod
}

type Option func(*pnaArchiveConfig)

func (p *pnaArchiveConfig) validate() error {
	if p.encryptionMethod != constants.NoEncryption && len(p.password) == 0 {
		return errors.New("pna: use encryption, but password is empty")
	}
	return nil
}

func defaultPnaArchiveConfig() *pnaArchiveConfig {
	return &pnaArchiveConfig{
		password:          "",
		encryptionMethod:  constants.NoEncryption,
		compressionMethod: constants.ZstdCompression,
	}
}

func Password(password string) Option {
	return func(p *pnaArchiveConfig) {
		p.password = password
	}
}
func Compression(method constants.CompressionMethod) Option {
	return func(p *pnaArchiveConfig) {
		p.compressionMethod = method
	}
}

func Encryption(method constants.EncryptionMethod) Option {
	return func(p *pnaArchiveConfig) {
		p.encryptionMethod = method
	}
}
