package main

import (
	"fmt"
	"io/ioutil"
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
	inputDir := "./pna"
	filepath.Walk(inputDir, func(path string, info os.FileInfo, err error) error {
		fmt.Println(path)
		if info.IsDir() {
			return nil
		}
		fhed := pna.NewFHEDChunk(
			pna.MajorVersion,
			pna.MinorVersion,
			pna.NoCompression,
			pna.NoEncryption,
			pna.FileTypeNormal,
			path,
		)
		data, err := ioutil.ReadFile(path)
		if err != nil {
			fmt.Print(err)
			return nil
		}
		fdat := pna.NewFDATChunk(data)
		fhed.WriteTo(wf)
		fdat.WriteTo(wf)
		pna.NewFENDChunk().WriteTo(wf)

		return nil
	})

	pna.NewAENDChunk().WriteTo(wf)
}
