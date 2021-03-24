package utils

import (
	"bytes"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/sha1"
	"encoding/binary"
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
	dist := make([]byte, len(src)-32)
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
	dist := make([]byte, len(src)-32)
	cipher.NewCBCDecrypter(ci, iv).CryptBlocks(dist, src[16+16:])
	dist, err = pkcs7unpad(dist, 16)
	if err != nil {
		return nil, err
	}
	return dist, nil
}
