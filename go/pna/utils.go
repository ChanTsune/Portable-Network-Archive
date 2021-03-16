package pna

import (
	"encoding/binary"
	"fmt"
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
