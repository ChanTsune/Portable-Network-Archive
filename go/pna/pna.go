package pna

var Header = []byte{
	0x89, // ¥x89
	0x50, // P
	0x4E, // N
	0x41, // A
	0x0D, // CR(Ctrl-M)
	0x0A, // LF(Ctrl-J)
	0x1A, // Ctrl-Z
	0x0A, // LF(Ctrl-J)
}

var MajorVersion = 0 // MajorVersion
var MinorVersion = 0 // MinorVersion
