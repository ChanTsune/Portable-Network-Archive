# Portable-Network-Archive

Portable-Network-Archive(PNA)
Highly scalable archive format with file compression, splitting and encryption.

## Data structure

The PNA consists of a structure called a `Chunk` except for the `Header`.  
Chunk has some `required chunks` and some `auxiliary chunks`.  
Unless otherwise specified, values are treated as signed integers.

### Header

The PNA is prefixed with the following header.

|hex|ASCII|
|--|--|
|0x89|¥x89|
|0x50|P|
|0x4E|N|
|0x41|A|
|0x0D|CR(Ctrl-M)|
|0x0A|LF(Ctrl-J)|
|0x1A|Ctrl-Z|
|0x0A|LF(Ctrl-J)|

### Chunk

It is represented by the following data structure called a chunk.

|name|size|description|
|--|--|--|
|Length|4-byte|Length of Chunk Data|
|Chunk Type|4-byte|Type of Chunk|
|Chunk Data|n-byte|Different interpretation of data depending on the type of chunk|
|CRC|4-byte|Crc32 calculated from Chunk Type and Chunk Data|

This is based on the PNG data structure.  
Byte order is big endian.  

### Required Chunks  

#### AHED  

The `AHED` chunk is stores basic information about the archive.

Chunk Data  

|significance|size|description|
|--|--|--|
|Major version|1-byte|Major version of PNA|
|Minor version|1-byte|Minor version of PNA|
|Archive number|4-byte|Archive number|
|Number of archives|4-byte|Number of archives|

##### Major version

It may be changed if there is a change in the structure of each chunk that makes up the PNA.
Currently only 0 is defined.

##### Minor version  

It may be changed when there is a change in the type of chunks that make up the PNA.
Currently only 0 is defined.

##### Archive number

Contains the number of the archive when the archive is split.  
0 if not split.

##### Number of archives

Contains the total number of split archives.
0 if not split.

#### ADAT

The `ADAT` also contains chunk structure data.

`ADAT` contains chunk structure data.  
Like PAN, these chunks make up the file data from several `required chunks` and `auxiliary chunks`.

##### FHED

Basic information of each file and directory is stored.  

|significance|size|description|
|--|--|--|
|Major version|1-byte|Major version|
|Minor version|1-byte|Minor version|
|Compression method|1-byte|Compression method|
|Encryption method|1-byte|Encryption method|
|File type|1-byte|file type|
|Null|1-byte|Separator|
|Path|n-byte|file path|

###### Compression method

The compression method is recorded.
0 is not compression
1 is deflate
2 is zstandard
4 is lzma

###### Encryption method

The encryption method is recorded.
0 is not encryption
1 is AES
2 is Camellia

###### File type

The file type is recorded.
0 is normal file
1 is directory
2 is symbolic link
4 is a file that has previously appeared in the archive

##### FDAT

The actual data of the file is recorded.

##### FEND

This signals the end of the file data stream.  
The chunk data area is empty.  

##### cTIM

File creation datetime are recorded in unix time.

##### mTIM

File last modified datetime are recorded in unix time.

##### fPRM

File permissions are recorded.

|significance|size|description|
|--|--|--|
|uid|8-byte|user ID|
|gid|8-byte|group ID|
|permissions|10-byte|file permission characters|

###### permissions

Unix file permission characters like `-rwxr-xr-x`.

#### AEND

The `AEND` chunk must appear last.  
This signals the end of the PNA data stream.  
The chunk data area is empty.  

### Auxiliary Chunks  

All Auxiliary Chunks must appear before the `AEND` Chunk

#### aTIM  

The last modified date of the archive is recorded in Unix time.  

|size|description|
|--|--|
|8byte|unix time stamp|
