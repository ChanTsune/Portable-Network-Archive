package main

import (
	"fmt"
	"io/ioutil"
	"log"
	"os"
	"path/filepath"
	"pna/pna"
	"pna/pna/chunk"
	"pna/pna/constants"
)

func main() {

	if err := pna.ExtractAll("./ext", "./sample.pna"); err != nil {
		fmt.Println(err.Error())
		return
	}

	wf, err := os.Create("./sample.pna")
	defer wf.Close()
	if err != nil {
		log.Fatalln(err)
	}
	wf.Write(constants.Header)
	chunk.NewAHEDChunk(
		constants.MajorVersion,
		constants.MinorVersion,
	).WriteTo(wf)
	inputDir := "./pna"
	filepath.Walk(inputDir, func(path string, info os.FileInfo, err error) error {
		fmt.Println(path)
		if info.IsDir() {
			return nil
		}
		fhed := chunk.NewFHEDChunk(
			constants.MajorVersion,
			constants.MinorVersion,
			constants.NoCompression,
			constants.NoEncryption,
			constants.FileTypeNormal,
			path,
		)
		data, err := ioutil.ReadFile(path)
		if err != nil {
			fmt.Print(err)
			return nil
		}
		fdat := chunk.NewFDATChunk(data)
		fhed.WriteTo(wf)
		fdat.WriteTo(wf)
		chunk.NewFENDChunk().WriteTo(wf)

		return nil
	})

	chunk.NewAENDChunk().WriteTo(wf)
}
