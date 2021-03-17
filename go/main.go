package main

import (
	"fmt"
	"pna/pna"
)

func main() {
	if err := pna.ArchiveAll("./pna", "./sample.pna"); err != nil {
		fmt.Println(err.Error())
		return
	}
	if err := pna.ExtractAll("./ext", "./sample.pna"); err != nil {
		fmt.Println(err.Error())
		return
	}
}
