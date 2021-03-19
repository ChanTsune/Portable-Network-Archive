package main

import (
	"errors"
	"log"
	"os"
	"pna/pna"

	"github.com/urfave/cli"
)

func main() {
	app := cli.NewApp()
	app.Name = "Potable-Network-Archive"
	app.Usage = ""
	app.Version = "0.0.0"

	app.Flags = []cli.Flag{
		cli.BoolFlag{
			Name:  "create, c",
			Usage: "create archive",
		},
		cli.BoolFlag{
			Name:  "extract, x",
			Usage: "extarct archive",
		},
	}

	app.Action = func(context *cli.Context) error {
		if context.Bool("c") {
			return archiveProcess(context)
		} else if context.Bool("x") {
			return extractProcess(context)
		}
		return cli.ShowAppHelp(context)
	}

	err := app.Run(os.Args)
	if err != nil {
		log.Fatalln(err)
	}
}

func extractProcess(context *cli.Context) error {
	archiveName := context.Args().First()
	if len(archiveName) == 0 {
		return errors.New("no files or directories specified")
	}
	if err := pna.ExtractAll("./ext", archiveName); err != nil {
		return err
	}
	return nil
}

func archiveProcess(context *cli.Context) error {
	archiveName := context.Args().First()
	if len(archiveName) == 0 {
		return errors.New("no files or directories specified")
	}
	if err := pna.ArchiveAll("./pna", archiveName); err != nil {
		return err
	}
	return nil
}
