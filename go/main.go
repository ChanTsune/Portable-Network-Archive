package main

import (
	"errors"
	"fmt"
	"log"
	"os"
	"pna/pna"
	"pna/pna/constants"

	"github.com/urfave/cli/v2"
)

type Option struct {
	EncryptionMethod  constants.EncryptionMethod
	CompressionMethod constants.CompressionMethod
}

func main() {
	app := cli.NewApp()
	app.Name = "Potable-Network-Archive"
	app.Usage = ""
	app.Version = "0.0.0"

	app.Flags = []cli.Flag{
		&cli.BoolFlag{
			Name:    "create",
			Aliases: []string{"c"},
			Usage:   "create archive",
		},
		&cli.BoolFlag{
			Name:    "extract",
			Aliases: []string{"x"},
			Usage:   "extarct archive",
		},
		&cli.StringFlag{
			Name:  "zip",
			Usage: "compression method. deflate, zstd and lzma is supported. or no is not compress",
			Value: "zstd",
		},
		&cli.StringFlag{
			Name:  "encrypt",
			Usage: "encryption method. aes and camellia is supported",
		},
		&cli.StringFlag{
			Name:  "password",
			Usage: "encryption/decryption password",
		},
		&cli.StringFlag{
			Name:    "output",
			Aliases: []string{"o"},
			Usage:   "output path",
		},
		// cli.BoolFlag{
		// 	Name:  "keep-permission",
		// 	Usage: "keep file permission",
		// },
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
	password := context.String("password")
	outputPath := context.String("output")
	if len(outputPath) == 0 {
		outputPath = "."
	}
	if err := pna.ExtractAll(outputPath, archiveName, password); err != nil {
		return err
	}
	return nil
}

func archiveProcess(context *cli.Context) error {
	archiveName := context.Args().First()
	if len(archiveName) == 0 {
		return errors.New("no files or directories specified")
	}
	argEncryptionMethod := context.String("encrypt")
	password := context.String("password")
	argCompressionMethod := context.String("zip")
	option := Option{
		EncryptionMethod:  constants.NoEncryption,
		CompressionMethod: constants.ZstdCompression,
	}
	switch argCompressionMethod {
	case "", "zstd":
		option.CompressionMethod = constants.ZstdCompression
	case "lzma":
		option.CompressionMethod = constants.LzmaCompression
	case "deflate":
		option.CompressionMethod = constants.DeflateCompression
	case "no":
		option.CompressionMethod = constants.NoCompression
	default:
		return fmt.Errorf("Unsupported compression method %s", argCompressionMethod)
	}
	if password != "" {
		switch argEncryptionMethod {
		case "", "aes":
			option.EncryptionMethod = constants.AesEncryption
		case "camellia":
			option.EncryptionMethod = constants.CamelliaEncryption
		default:
			return fmt.Errorf("Unsupported encryption method %s", argEncryptionMethod)
		}
	} else {
		switch argEncryptionMethod {
		case "", "aes", "camellia":
		default:
			return fmt.Errorf("Unsupported encryption method %s", argEncryptionMethod)
		}
	}
	if err := pna.Archive(
		archiveName,
		context.Args().Slice()[1:],
		pna.Compression(option.CompressionMethod),
		pna.Encryption(option.EncryptionMethod),
		pna.Password(password),
	); err != nil {
		return err
	}
	return nil
}
