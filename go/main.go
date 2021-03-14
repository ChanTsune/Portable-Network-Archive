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
		uint8(pna.MajorVersion),
		uint8(pna.MinorVersion),
	).WriteTo(wf)

	filepath.Walk(".", func(path string, info os.FileInfo, err error) error {
		fmt.Println(path, info, err)
		return nil
	})

	pna.NewAENDChunk().WriteTo(wf)
}
