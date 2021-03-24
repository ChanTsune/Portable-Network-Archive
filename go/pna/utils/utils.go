package utils

import (
	"crypto/rand"
	"encoding/binary"
	"fmt"
	"math/big"
	"strings"
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
