package main

import (
	"fmt"
	"log"
	"os"
	"path/filepath"
	"pna/pna"
)

func main() {
	wf, err := os.Create("./sample.pna")
	defer wf.Close()
	if err != nil {
		log.Fatalln(err)
	}
	wf.Write(pna.Header)
	pna.NewAHEDChunk(
		pna.MajorVersion,
		pna.MinorVersion,
	).WriteTo(wf)

	filepath.Walk(".", func(path string, info os.FileInfo, err error) error {
		fmt.Println(path, info, err)
		pna.NewFHEDChunk(
			pna.MajorVersion,
			pna.MinorVersion,
			pna.NoCompression,
			pna.NoEncryption,
			pna.FileTypeNormal,
			path,
		)
		return nil
	})

	pna.NewAENDChunk().WriteTo(wf)
}
