package utils

import (
	"bytes"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/sha1"
	"encoding/binary"
	"errors"
	"fmt"
	"math/big"
	"strings"

	"github.com/dgryski/go-camellia"
	"golang.org/x/crypto/pbkdf2"
)

func Uint32ToBytes(n uint32) []byte {
	r := make([]byte, 4)
	binary.BigEndian.PutUint32(r, n)
	return r
}

func Uint16ToBytes(n uint16) []byte {
	r := make([]byte, 2)
	binary.BigEndian.PutUint16(r, n)
	return r
}

// Bytes2str converts []byte to string("00 00 00 00 00 00 00 00")
func Bytes2str(bytes ...byte) string {
	strs := []string{}
	for _, b := range bytes {
		strs = append(strs, fmt.Sprintf("%02x", b))
	}
	return strings.Join(strs, " ")
}

func randUint8() uint8 {
	i, err := rand.Int(rand.Reader, big.NewInt(128))
	if err != nil {
		panic(err.Error())
	}
	return uint8(i.Int64())
}

func RandBytes(n int) []byte {
	// TODO: Uint8 to Uint64
	b := make([]byte, n)
	for i := 0; i < n; i++ {
		b[i] = randUint8()
	}
	return b
}

func StretchPassword(password string, salt []byte) []byte {
	return pbkdf2.Key([]byte(password), salt, 10000, 32, sha1.New)
}

func CamelliaEncryption(src []byte, password string) ([]byte, error) {
	salt := RandBytes(16)
	pwd := StretchPassword(password, salt)
	ci, err := camellia.New(pwd)
	if err != nil {
		return nil, err
	}
	iv := RandBytes(16)
	src, err = pkcs7pad(src, 16)
	if err != nil {
		return nil, err
	}
	dist := make([]byte, len(src))
	cipher.NewCBCEncrypter(ci, iv).CryptBlocks(dist, src)
	return bytes.Join([][]byte{salt, iv, dist}, []byte{}), nil
}

func CamelliaDecryption(src []byte, password string) ([]byte, error) {
	salt := src[:16]
	pwd := StretchPassword(password, salt)
	ci, err := camellia.New(pwd)
	if err != nil {
		return nil, err
	}
	iv := src[16 : 16+16]
	dist := make([]byte, len(src))
	cipher.NewCBCDecrypter(ci, iv).CryptBlocks(dist, src[16+16:])
	dist, err = pkcs7unpad(dist, 16)
	if err != nil {
		return nil, err
	}
	return dist, nil
}

func AesEncryption(src []byte, password string) ([]byte, error) {
	salt := RandBytes(16)
	pwd := StretchPassword(password, salt)
	ci, err := aes.NewCipher(pwd)
	if err != nil {
		return nil, err
	}
	iv := RandBytes(16)
	src, err = pkcs7pad(src, 16)
	if err != nil {
		return nil, err
	}
	dist := make([]byte, len(src))
	cipher.NewCBCEncrypter(ci, iv).CryptBlocks(dist, src)
	return bytes.Join([][]byte{salt, iv, dist}, []byte{}), nil
}

func AESDecryption(src []byte, password string) ([]byte, error) {
	salt := src[:16]
	pwd := StretchPassword(password, salt)
	ci, err := aes.NewCipher(pwd)
	if err != nil {
		return nil, err
	}
	iv := src[16 : 16+16]
	dist := make([]byte, len(src))
	cipher.NewCBCDecrypter(ci, iv).CryptBlocks(dist, src[16+16:])
	dist, err = pkcs7unpad(dist, 16)
	if err != nil {
		return nil, err
	}
	return dist, nil
}

// pkcs7unpad remove pkcs7 padding
func pkcs7unpad(data []byte, blockSize int) ([]byte, error) {
	length := len(data)
	if length == 0 {
		return nil, errors.New("pkcs7: Data is empty")
	}
	if length%blockSize != 0 {
		return nil, errors.New("pkcs7: Data is not block-aligned")
	}
	padLen := int(data[length-1])
	ref := bytes.Repeat([]byte{byte(padLen)}, padLen)
	if padLen > blockSize || padLen == 0 || !bytes.HasSuffix(data, ref) {
		return nil, errors.New("pkcs7: Invalid padding")
	}
	return data[:length-padLen], nil
}

// pkcs7pad add pkcs7 padding
func pkcs7pad(data []byte, blockSize int) ([]byte, error) {
	if blockSize < 0 || blockSize > 256 {
		return nil, fmt.Errorf("pkcs7: Invalid block size %d", blockSize)
	} else {
		padLen := blockSize - len(data)%blockSize
		padding := bytes.Repeat([]byte{byte(padLen)}, padLen)
		return append(data, padding...), nil
	}
}
