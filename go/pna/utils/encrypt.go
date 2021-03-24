package utils

import (
	"bytes"
	"crypto/aes"
	"crypto/cipher"
	"crypto/sha1"

	"github.com/dgryski/go-camellia"
	"golang.org/x/crypto/pbkdf2"
)

func StretchPassword(password string, salt []byte) []byte {
	return pbkdf2.Key([]byte(password), salt, 10000, 32, sha1.New)
}

func CamelliaEncryption(src []byte, password string) ([]byte, error) {
	blockSize := camellia.BlockSize
	salt := RandBytes(blockSize)
	pwd := StretchPassword(password, salt)
	ci, err := camellia.New(pwd)
	if err != nil {
		return nil, err
	}
	iv := RandBytes(blockSize)
	src, err = pkcs7pad(src, blockSize)
	if err != nil {
		return nil, err
	}
	dist := make([]byte, len(src))
	cipher.NewCBCEncrypter(ci, iv).CryptBlocks(dist, src)
	return bytes.Join([][]byte{salt, iv, dist}, []byte{}), nil
}

func CamelliaDecryption(src []byte, password string) ([]byte, error) {
	blockSize := camellia.BlockSize
	salt := src[:blockSize]
	pwd := StretchPassword(password, salt)
	ci, err := camellia.New(pwd)
	if err != nil {
		return nil, err
	}
	iv := src[blockSize : blockSize*2]
	dist := make([]byte, len(src)-blockSize*2)
	cipher.NewCBCDecrypter(ci, iv).CryptBlocks(dist, src[blockSize*2:])
	dist, err = pkcs7unpad(dist, blockSize)
	if err != nil {
		return nil, err
	}
	return dist, nil
}

func AesEncryption(src []byte, password string) ([]byte, error) {
	blockSize := aes.BlockSize
	salt := RandBytes(blockSize)
	pwd := StretchPassword(password, salt)
	ci, err := aes.NewCipher(pwd)
	if err != nil {
		return nil, err
	}
	iv := RandBytes(blockSize)
	src, err = pkcs7pad(src, blockSize)
	if err != nil {
		return nil, err
	}
	dist := make([]byte, len(src))
	cipher.NewCBCEncrypter(ci, iv).CryptBlocks(dist, src)
	return bytes.Join([][]byte{salt, iv, dist}, []byte{}), nil
}

func AESDecryption(src []byte, password string) ([]byte, error) {
	blockSize := aes.BlockSize
	salt := src[:blockSize]
	pwd := StretchPassword(password, salt)
	ci, err := aes.NewCipher(pwd)
	if err != nil {
		return nil, err
	}
	iv := src[blockSize : blockSize*2]
	dist := make([]byte, len(src)-blockSize*2)
	cipher.NewCBCDecrypter(ci, iv).CryptBlocks(dist, src[blockSize*2:])
	dist, err = pkcs7unpad(dist, blockSize)
	if err != nil {
		return nil, err
	}
	return dist, nil
}
