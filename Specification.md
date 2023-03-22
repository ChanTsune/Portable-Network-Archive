# Portable-Network-Archive

Portable-Network-Archive (PNA)  
Highly scalable archive format based on the PNG data structure with file compression, splitting and encryption.

## Data structure

The PNA consists of a structure called a `Chunk` except for the `Header`.  
Chunk has some `required chunks` and some `auxiliary chunks`.  
Unless otherwise specified, values are treated as signed integers.

### Header

The PNA is prefixed with the following header.

|  hex  |    ASCII    |
|:-----:|:-----------:|
| 0x89  |    ¥x89     |
| 0x50  |      P      |
| 0x4E  |      N      |
| 0x41  |      A      |
| 0x0D  | CR(Ctrl-M)  |
| 0x0A  | LF(Ctrl-J)  |
| 0x1A  |   Ctrl-Z    |
| 0x0A  | LF(Ctrl-J)  |

### Chunk

It is represented by the following data structure called a chunk.

| name       |  size   | description                                                     |
|:-----------|:-------:|:----------------------------------------------------------------|
| Length     | 4-byte  | Length of Chunk Data                                            |
| Chunk Type | 4-byte  | Type of Chunk                                                   |
| Chunk Data | n-byte  | Different interpretation of data depending on the type of chunk |
| CRC        | 4-byte  | Crc32 calculated from Chunk Type and Chunk Data                 |

This is based on the PNG data structure.  
Byte order is big endian.  

### Critical Chunks  

#### AHED  

The `AHED` chunk is stores basic information about the archive.
All valid chunks must appear between this chunk and the `AEND` chunk described below.

Chunk Data  

| significance             |  size   | description          |
|:-------------------------|:-------:|:---------------------|
| Major version            | 1-byte  | Major version of PNA |
| Minor version            | 1-byte  | Minor version of PNA |
| General purpose bit flag | 2-byte  | Bit flags            |
| Archive number           | 4-byte  | Archive number       |

##### Major version

It may be changed if there is a change in the structure of each chunk that makes up the PNA.
Currently only 0 is defined.

##### Minor version  

It may be changed when there is a change in the type of chunks that make up the PNA.
Currently only 0 is defined.

##### General purpose bit flag

__Bit0__ Use solid mode.

__Bit1__ ~ __Bit15__ currently dose not used. reserve for future.

##### Archive number

Contains the number of the archive when the archive is split.  
Archive number is start with 0.

#### AEND

The `AEND` chunk must appear last.  
This signals the end of the PNA data stream.  
No more than this chunk should be loaded.  
The chunk data area is empty.

#### ANXT

Indicates that the archive is split and the following file exists.
The Archive number field of the `AHED` chunk of the next file will be the value of the Archive number field of the `AHED` chunk of the current file incremented by 1.
The chunk data area is empty.

#### FHED

Basic information of each file and directory is stored.  

|significance|size|description|
|--|--|--|
|Major version|1-byte|Major version|
|Minor version|1-byte|Minor version|
|File type|1-byte|file type|
|Compression method|1-byte|Compression method|
|Encryption method|1-byte|Encryption method|
|Cipher mode|1-byte|Cipher mode|
|Path|n-byte|file path|

##### File type

The file type is recorded.
0 is regular file
1 is directory
2 is symbolic link
3 is hard link
4 is a file that has previously appeared in the archive

##### Compression method

The compression method is recorded.
0 is not compression
1 is deflate
2 is zstandard
4 is lzma

##### Encryption method

The encryption method is recorded.
0 is not encryption
1 is AES
2 is Camellia

When this field value is 0, `PHSF` chunk is not required.  

##### Cipher mode

Cipher mode of encryption.
0 is cbc mode
1 is ctr mode

##### File path
File path must be utf-8 encoded string.

#### PHSF

The information about the key derivation function when encrypting a file.  
This chunk appeared after `FHAD` chunk and before `FDAT` chunk.  
If the value of encryption method field of `FHAD` chunk is not 0, this chunk is required.  

|size|description|
|--|--|
|n-byte|PHC string format|

About [PHC string format](https://github.com/P-H-C/phc-string-format/blob/master/phc-sf-spec.md)

#### FDAT

The actual data of the file is recorded.

#### FEND

This signals the end of the file data stream.  
The chunk data area is empty.  

### Auxiliary Chunks  

All Auxiliary Chunks must appear before the `AEND` Chunk

#### cTIM

The creation datetime is recorded in unix time.
When this chunk appears after the `FHAD` chunk and before the `FEND` chunk, it indicates the creation datetime of the file.

|  size  | description     |
|:------:|:----------------|
| 8byte  | unix time stamp |

#### mTIM

The last modified datetime is recorded in unix time.
When this chunk appears after the `FHAD` chunk and before the `FEND` chunk, it indicates the last modified datetime of the file.

|  size  | description     |
|:------:|:----------------|
| 8byte  | unix time stamp |

#### fPRM

File permissions are recorded.
This chunk appeared after `FHAD` chunk and before `FEND` chunk.

| significance |  size  | description           |
|:-------------|:------:|:----------------------|
| uid          | 8-byte | user ID               |
| uname length | 1-byte | length of uname       |
| uname        | n-byte | unix user name        |
| gid          | 8-byte | group ID              |
| gname length | 1-byte | length of gname       |
| gname        | n-byte | unix group name       |
| permissions  | 2-byte | file permission bytes |

##### permissions

Permissions are like `755` as use in `chmod`.

#### aSLD

Basic information of Solid mode archive is stored.  

|significance|size|description|
|--|--|--|
|Major version|1-byte|Major version|
|Minor version|1-byte|Minor version|
|Compression method|1-byte|Compression method|
|Encryption method|1-byte|Encryption method|

#### aDAT

Solid mode archive data.
