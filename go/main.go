package main

import (
	"os"
	"pna/pna"

	"github.com/urfave/cli"
)

func main() {
	app := cli.NewApp()
	app.Name = "Potable-Network-Archive"
	app.Version = "0.0.0"

	app.Action = func(context *cli.Context) error {
		return cli.ShowAppHelp(context)
	}

	err := app.Run(os.Args)
	if err != nil {
		println(err.Error())
	}
}

func extractProcess(context *cli.Context) error {
	if err := pna.ExtractAll("./ext", "./sample.pna"); err != nil {
		return err
	}
	return nil
}

func archiveProcess(context *cli.Context) error {
	if err := pna.ArchiveAll("./pna", "./sample.pna"); err != nil {
		return err
	}
	return nil
}
