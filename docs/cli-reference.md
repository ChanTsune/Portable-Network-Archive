# Command-Line Help for `pna`

This document contains the help content for the `pna` command-line program.

**Command Overview:**

* [`pna`↴](#pna)
* [`pna create`↴](#pna-create)
* [`pna append`↴](#pna-append)
* [`pna extract`↴](#pna-extract)
* [`pna list`↴](#pna-list)
* [`pna split`↴](#pna-split)
* [`pna concat`↴](#pna-concat)
* [`pna strip`↴](#pna-strip)
* [`pna sort`↴](#pna-sort)
* [`pna xattr`↴](#pna-xattr)
* [`pna xattr get`↴](#pna-xattr-get)
* [`pna xattr set`↴](#pna-xattr-set)
* [`pna xattr help`↴](#pna-xattr-help)
* [`pna xattr help get`↴](#pna-xattr-help-get)
* [`pna xattr help set`↴](#pna-xattr-help-set)
* [`pna xattr help help`↴](#pna-xattr-help-help)
* [`pna complete`↴](#pna-complete)
* [`pna bug-report`↴](#pna-bug-report)
* [`pna experimental`↴](#pna-experimental)
* [`pna experimental stdio`↴](#pna-experimental-stdio)
* [`pna experimental delete`↴](#pna-experimental-delete)
* [`pna experimental update`↴](#pna-experimental-update)
* [`pna experimental chown`↴](#pna-experimental-chown)
* [`pna experimental chmod`↴](#pna-experimental-chmod)
* [`pna experimental xattr`↴](#pna-experimental-xattr)
* [`pna experimental xattr get`↴](#pna-experimental-xattr-get)
* [`pna experimental xattr set`↴](#pna-experimental-xattr-set)
* [`pna experimental xattr help`↴](#pna-experimental-xattr-help)
* [`pna experimental xattr help get`↴](#pna-experimental-xattr-help-get)
* [`pna experimental xattr help set`↴](#pna-experimental-xattr-help-set)
* [`pna experimental xattr help help`↴](#pna-experimental-xattr-help-help)
* [`pna experimental acl`↴](#pna-experimental-acl)
* [`pna experimental acl get`↴](#pna-experimental-acl-get)
* [`pna experimental acl set`↴](#pna-experimental-acl-set)
* [`pna experimental acl help`↴](#pna-experimental-acl-help)
* [`pna experimental acl help get`↴](#pna-experimental-acl-help-get)
* [`pna experimental acl help set`↴](#pna-experimental-acl-help-set)
* [`pna experimental acl help help`↴](#pna-experimental-acl-help-help)
* [`pna experimental migrate`↴](#pna-experimental-migrate)
* [`pna experimental chunk`↴](#pna-experimental-chunk)
* [`pna experimental chunk list`↴](#pna-experimental-chunk-list)
* [`pna experimental chunk help`↴](#pna-experimental-chunk-help)
* [`pna experimental chunk help list`↴](#pna-experimental-chunk-help-list)
* [`pna experimental chunk help help`↴](#pna-experimental-chunk-help-help)
* [`pna experimental sort`↴](#pna-experimental-sort)
* [`pna experimental diff`↴](#pna-experimental-diff)
* [`pna experimental help`↴](#pna-experimental-help)
* [`pna experimental help stdio`↴](#pna-experimental-help-stdio)
* [`pna experimental help delete`↴](#pna-experimental-help-delete)
* [`pna experimental help update`↴](#pna-experimental-help-update)
* [`pna experimental help chown`↴](#pna-experimental-help-chown)
* [`pna experimental help chmod`↴](#pna-experimental-help-chmod)
* [`pna experimental help xattr`↴](#pna-experimental-help-xattr)
* [`pna experimental help xattr get`↴](#pna-experimental-help-xattr-get)
* [`pna experimental help xattr set`↴](#pna-experimental-help-xattr-set)
* [`pna experimental help acl`↴](#pna-experimental-help-acl)
* [`pna experimental help acl get`↴](#pna-experimental-help-acl-get)
* [`pna experimental help acl set`↴](#pna-experimental-help-acl-set)
* [`pna experimental help migrate`↴](#pna-experimental-help-migrate)
* [`pna experimental help chunk`↴](#pna-experimental-help-chunk)
* [`pna experimental help chunk list`↴](#pna-experimental-help-chunk-list)
* [`pna experimental help sort`↴](#pna-experimental-help-sort)
* [`pna experimental help diff`↴](#pna-experimental-help-diff)
* [`pna experimental help help`↴](#pna-experimental-help-help)
* [`pna help`↴](#pna-help)
* [`pna help create`↴](#pna-help-create)
* [`pna help append`↴](#pna-help-append)
* [`pna help extract`↴](#pna-help-extract)
* [`pna help list`↴](#pna-help-list)
* [`pna help split`↴](#pna-help-split)
* [`pna help concat`↴](#pna-help-concat)
* [`pna help strip`↴](#pna-help-strip)
* [`pna help sort`↴](#pna-help-sort)
* [`pna help xattr`↴](#pna-help-xattr)
* [`pna help xattr get`↴](#pna-help-xattr-get)
* [`pna help xattr set`↴](#pna-help-xattr-set)
* [`pna help complete`↴](#pna-help-complete)
* [`pna help bug-report`↴](#pna-help-bug-report)
* [`pna help experimental`↴](#pna-help-experimental)
* [`pna help experimental stdio`↴](#pna-help-experimental-stdio)
* [`pna help experimental delete`↴](#pna-help-experimental-delete)
* [`pna help experimental update`↴](#pna-help-experimental-update)
* [`pna help experimental chown`↴](#pna-help-experimental-chown)
* [`pna help experimental chmod`↴](#pna-help-experimental-chmod)
* [`pna help experimental xattr`↴](#pna-help-experimental-xattr)
* [`pna help experimental xattr get`↴](#pna-help-experimental-xattr-get)
* [`pna help experimental xattr set`↴](#pna-help-experimental-xattr-set)
* [`pna help experimental acl`↴](#pna-help-experimental-acl)
* [`pna help experimental acl get`↴](#pna-help-experimental-acl-get)
* [`pna help experimental acl set`↴](#pna-help-experimental-acl-set)
* [`pna help experimental migrate`↴](#pna-help-experimental-migrate)
* [`pna help experimental chunk`↴](#pna-help-experimental-chunk)
* [`pna help experimental chunk list`↴](#pna-help-experimental-chunk-list)
* [`pna help experimental sort`↴](#pna-help-experimental-sort)
* [`pna help experimental diff`↴](#pna-help-experimental-diff)
* [`pna help help`↴](#pna-help-help)

## `pna`

Portable-Network-Archive cli

**Usage:** `pna [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `create` — Create archive
* `append` — Append files to archive
* `extract` — Extract files from archive
* `list` — List files in archive
* `split` — Split archive
* `concat` — Concat archives
* `strip` — Strip entries metadata
* `sort` — Sort entries in archive
* `xattr` — Manipulate extended attributes
* `complete` — Generate shell auto complete
* `bug-report` — Generate bug report template
* `experimental` — Unstable experimental commands; behavior and interface may change or be removed
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help
* `-V`, `--version` — Print version



## `pna create`

Create archive

**Usage:** `pna create [OPTIONS] <--file <FILE>|ARCHIVE> [FILES]...`

**Command Alias:** `c`

###### **Arguments:**

* `<ARCHIVE>`
* `<FILES>`

###### **Options:**

* `--one-file-system` — Stay in the same file system when collecting files (unstable)

  Default value: `false`
* `--nodump` — Exclude files with the nodump flag (unstable)

  Default value: `false`
* `-r`, `--recursive` [alias: `recursion`] — Add the directory to the archive recursively

  Default value: `true`
* `--no-recursive` [alias: `no-recursion`] — Do not recursively add directories to the archives. This is the inverse option of --recursive

  Default value: `false`
* `--overwrite` — Overwrite file

  Default value: `false`
* `--no-overwrite` — Do not overwrite files. This is the inverse option of --overwrite

  Default value: `false`
* `--keep-dir` — Archiving the directories

  Default value: `false`
* `--no-keep-dir` — Do not archive directories. This is the inverse option of --keep-dir

  Default value: `false`
* `--keep-timestamp` [alias: `preserve-timestamps`] — Archiving the timestamp of the files

  Default value: `false`
* `--no-keep-timestamp` [alias: `no-preserve-timestamps`] — Do not archive timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Archiving the permissions of the files (unstable on Windows)

  Default value: `false`
* `--no-keep-permission` [alias: `no-preserve-permissions`] — Do not archive permissions of files. This is the inverse option of --preserve-permissions

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Archiving the extended attributes of the files

  Default value: `false`
* `--no-keep-xattr` [alias: `no-preserve-xattrs`] — Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Archiving the acl of the files (unstable)

  Default value: `false`
* `--no-keep-acl` [alias: `no-preserve-acls`] — Do not archive acl of files. This is the inverse option of --keep-acl (unstable)

  Default value: `false`
* `--split <size>` — Splits archive by given size in bytes (minimum 64B)
* `--solid` — Create an archive in solid mode

  Default value: `false`
* `--uname <UNAME>` — Archiving user to the entries from given name
* `--gname <GNAME>` — Archiving group to the entries from given name
* `--uid <UID>` — Overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id
* `--gid <GID>` — Overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id
* `--strip-components <STRIP_COMPONENTS>` — Remove the specified number of leading path elements when storing paths (unstable)
* `--numeric-owner` — This is equivalent to --uname "" --gname "". It causes user and group names to not be stored in the archive

  Default value: `false`
* `--ctime <CTIME>` — Overrides the creation time read from disk
* `--clamp-ctime` — Clamp the creation time of the entries to the specified time by --ctime

  Default value: `false`
* `--atime <ATIME>` — Overrides the access time read from disk
* `--clamp-atime` — Clamp the access time of the entries to the specified time by --atime

  Default value: `false`
* `--mtime <MTIME>` — Overrides the modification time read from disk
* `--clamp-mtime` — Clamp the modification time of the entries to the specified time by --mtime

  Default value: `false`
* `--older-ctime <OLDER_CTIME>` — Only include files and directories older than the specified date (unstable). This compares ctime entries.
* `--older-mtime <OLDER_MTIME>` — Only include files and directories older than the specified date (unstable). This compares mtime entries.
* `--newer-ctime <NEWER_CTIME>` — Only include files and directories newer than the specified date (unstable). This compares ctime entries.
* `--newer-mtime <NEWER_MTIME>` — Only include files and directories newer than the specified date (unstable). This compares mtime entries.
* `--newer-ctime-than <file>` — Only include files and directories newer than the specified file (unstable). This compares ctime entries.
* `--newer-mtime-than <file>` — Only include files and directories newer than the specified file (unstable). This compares mtime entries.
* `--older-ctime-than <file>` — Only include files and directories older than the specified file (unstable). This compares ctime entries.
* `--older-mtime-than <file>` — Only include files and directories older than the specified file (unstable). This compares mtime entries.
* `--files-from <FILES_FROM>` — Read archiving files from given path (unstable)
* `--files-from-stdin` — Read archiving files from stdin (unstable)

  Default value: `false`
* `--include <INCLUDE>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)
* `--exclude <EXCLUDE>` — Exclude path glob (unstable)
* `--exclude-from <EXCLUDE_FROM>` — Read exclude files from given path (unstable)
* `--exclude-vcs` — Exclude vcs files (unstable)

  Default value: `false`
* `--gitignore` — Ignore files from .gitignore (unstable)

  Default value: `false`
* `--follow-links` [alias: `dereference`] — Follow symbolic links

  Default value: `false`
* `-H`, `--follow-command-links` — Follow symbolic links named on the command line

  Default value: `false`
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option (unstable)
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option (unstable)
* `-C`, `--cd <DIRECTORY>` [alias: `directory`] — changes the directory before adding the following files
* `--store` — No compression

  Default value: `false`
* `--deflate <level>` — Use deflate for compression [possible level: 1-9, min, max]
* `--zstd <level>` — Use zstd for compression [possible level: 1-21, min, max]
* `--xz <level>` — Use xz for compression [possible level: 0-9, min, max]
* `--aes <cipher mode>` — Use aes for encryption

  Possible values: `cbc`, `ctr`

* `--camellia <cipher mode>` — Use camellia for encryption

  Possible values: `cbc`, `ctr`

* `--argon2 <ARGON2>` — Use argon2 for password hashing
* `--pbkdf2 <PBKDF2>` — Use pbkdf2 for password hashing
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `-f`, `--file <FILE>`
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna append`

Append files to archive

**Usage:** `pna append [OPTIONS] <--file <FILE>|ARCHIVE> [FILES]...`

**Command Alias:** `a`

###### **Arguments:**

* `<ARCHIVE>`
* `<FILES>`

###### **Options:**

* `--one-file-system` — Stay in the same file system when collecting files (unstable)

  Default value: `false`
* `--nodump` — Exclude files with the nodump flag (unstable)

  Default value: `false`
* `-r`, `--recursive` [alias: `recursion`] — Add the directory to the archive recursively

  Default value: `true`
* `--no-recursive` [alias: `no-recursion`] — Do not recursively add directories to the archives. This is the inverse option of --recursive

  Default value: `false`
* `--keep-dir` — Archiving the directories

  Default value: `false`
* `--no-keep-dir` — Do not archive directories. This is the inverse option of --keep-dir

  Default value: `false`
* `--keep-timestamp` [alias: `preserve-timestamps`] — Archiving the timestamp of the files

  Default value: `false`
* `--no-keep-timestamp` [alias: `no-preserve-timestamps`] — Do not archive timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Archiving the permissions of the files (unstable on Windows)

  Default value: `false`
* `--no-keep-permission` [alias: `no-preserve-permissions`] — Do not archive permissions of files. This is the inverse option of --preserve-permissions

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Archiving the extended attributes of the files

  Default value: `false`
* `--no-keep-xattr` [alias: `no-preserve-xattrs`] — Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Archiving the acl of the files (unstable)

  Default value: `false`
* `--no-keep-acl` [alias: `no-preserve-acls`] — Do not archive acl of files. This is the inverse option of --keep-acl (unstable)

  Default value: `false`
* `--uname <UNAME>` — Archiving user to the entries from given name
* `--gname <GNAME>` — Archiving group to the entries from given name
* `--uid <UID>` — Overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id
* `--gid <GID>` — Overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id
* `--strip-components <STRIP_COMPONENTS>` — Remove the specified number of leading path elements when storing paths (unstable)
* `--numeric-owner` — This is equivalent to --uname "" --gname "". It causes user and group names to not be stored in the archive

  Default value: `false`
* `--ctime <CTIME>` — Overrides the creation time read from disk
* `--clamp-ctime` — Clamp the creation time of the entries to the specified time by --ctime

  Default value: `false`
* `--atime <ATIME>` — Overrides the access time read from disk
* `--clamp-atime` — Clamp the access time of the entries to the specified time by --atime

  Default value: `false`
* `--mtime <MTIME>` — Overrides the modification time read from disk
* `--clamp-mtime` — Clamp the modification time of the entries to the specified time by --mtime

  Default value: `false`
* `--older-ctime <OLDER_CTIME>` — Only include files and directories older than the specified date (unstable). This compares ctime entries.
* `--older-mtime <OLDER_MTIME>` — Only include files and directories older than the specified date (unstable). This compares mtime entries.
* `--newer-ctime <NEWER_CTIME>` — Only include files and directories newer than the specified date (unstable). This compares ctime entries.
* `--newer-mtime <NEWER_MTIME>` — Only include files and directories newer than the specified date (unstable). This compares mtime entries.
* `--newer-ctime-than <file>` — Only include files and directories newer than the specified file (unstable). This compares ctime entries.
* `--newer-mtime-than <file>` — Only include files and directories newer than the specified file (unstable). This compares mtime entries.
* `--older-ctime-than <file>` — Only include files and directories older than the specified file (unstable). This compares ctime entries.
* `--older-mtime-than <file>` — Only include files and directories older than the specified file (unstable). This compares mtime entries.
* `--files-from <FILES_FROM>` — Read archiving files from given path (unstable)
* `--files-from-stdin` — Read archiving files from stdin (unstable)

  Default value: `false`
* `--include <INCLUDE>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)
* `--exclude <EXCLUDE>` — Exclude path glob (unstable)
* `--exclude-from <EXCLUDE_FROM>` — Read exclude files from given path (unstable)
* `--exclude-vcs` — Exclude vcs files (unstable)

  Default value: `false`
* `--gitignore` — Ignore files from .gitignore (unstable)

  Default value: `false`
* `--follow-links` [alias: `dereference`] — Follow symbolic links

  Default value: `false`
* `-H`, `--follow-command-links` — Follow symbolic links named on the command line

  Default value: `false`
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option (unstable)
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option (unstable)
* `-C`, `--cd <DIRECTORY>` [alias: `directory`] — changes the directory before adding the following files
* `--store` — No compression

  Default value: `false`
* `--deflate <level>` — Use deflate for compression [possible level: 1-9, min, max]
* `--zstd <level>` — Use zstd for compression [possible level: 1-21, min, max]
* `--xz <level>` — Use xz for compression [possible level: 0-9, min, max]
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--aes <cipher mode>` — Use aes for encryption

  Possible values: `cbc`, `ctr`

* `--camellia <cipher mode>` — Use camellia for encryption

  Possible values: `cbc`, `ctr`

* `--argon2 <ARGON2>` — Use argon2 for password hashing
* `--pbkdf2 <PBKDF2>` — Use pbkdf2 for password hashing
* `-f`, `--file <FILE>`
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna extract`

Extract files from archive

**Usage:** `pna extract [OPTIONS] <--file <FILE>|ARCHIVE> [FILES]...`

**Command Alias:** `x`

###### **Arguments:**

* `<ARCHIVE>`
* `<FILES>`

###### **Options:**

* `--overwrite` — Overwrite file

  Default value: `false`
* `--no-overwrite` — Do not overwrite files. This is the inverse option of --overwrite

  Default value: `false`
* `--keep-newer-files` — Skip extracting files if a newer version already exists

  Default value: `false`
* `--keep-old-files` — Skip extracting files if they already exist

  Default value: `false`
* `--out-dir <OUT_DIR>` — Output directory of extracted files
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--keep-timestamp` [alias: `preserve-timestamps`] — Restore the timestamp of the files

  Default value: `false`
* `--no-keep-timestamp` [alias: `no-preserve-timestamps`] — Do not restore timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Restore the permissions of the files (unstable on Windows)

  Default value: `false`
* `--no-keep-permission` [alias: `no-preserve-permissions`] — Do not restore permissions of files. This is the inverse option of --preserve-permissions

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Restore the extended attributes of the files

  Default value: `false`
* `--no-keep-xattr` [alias: `no-preserve-xattrs`] — Do not restore extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Restore the acl of the files (unstable)

  Default value: `false`
* `--no-keep-acl` [alias: `no-preserve-acls`] — Do not restore acl of files. This is the inverse option of --keep-acl (unstable)

  Default value: `false`
* `--uname <UNAME>` — Restore user from given name
* `--gname <GNAME>` — Restore group from given name
* `--uid <UID>` — Overrides the user id in the archive; the user name in the archive will be ignored
* `--gid <GID>` — Overrides the group id in the archive; the group name in the archive will be ignored
* `--numeric-owner` — This is equivalent to --uname "" --gname "". It causes user and group names in the archive to be ignored in favor of the numeric user and group ids.

  Default value: `false`
* `--include <INCLUDE>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)
* `--exclude <EXCLUDE>` — Exclude path glob (unstable)
* `--exclude-from <EXCLUDE_FROM>` — Read exclude files from given path (unstable)
* `--exclude-vcs` — Exclude vcs files (unstable)

  Default value: `false`
* `--files-from <FILES_FROM>` — Read extraction patterns from given path (unstable)
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `--strip-components <STRIP_COMPONENTS>` — Remove the specified number of leading path elements. Path names with fewer elements will be silently skipped
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option (unstable)
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option (unstable)
* `--same-owner` — Try extracting files with the same ownership as exists in the archive

  Default value: `false`
* `--no-same-owner` — Extract files as yourself

  Default value: `false`
* `-C`, `--cd <DIRECTORY>` [alias: `directory`] — Change directories after opening the archive but before extracting entries from the archive
* `--chroot` — chroot() to the current directory after processing any --cd options and before extracting any files

  Default value: `false`
* `--allow-unsafe-links` — Allow extracting symbolic links and hard links that contain root or parent paths

  Default value: `false`
* `-f`, `--file <FILE>`
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna list`

List files in archive

**Usage:** `pna list [OPTIONS] <--file <FILE>|ARCHIVE> [FILES]...`

**Command Aliases:** `l`, `ls`

###### **Arguments:**

* `<ARCHIVE>`
* `<FILES>`

###### **Options:**

* `-l`, `--long` — Display extended file metadata as a table

  Default value: `false`
* `-h`, `--header` — Add a header row to each column

  Default value: `false`
* `--solid` — Display solid mode archive entries

  Default value: `false`
* `-@` — Display extended file attributes in a table

  Default value: `false`
* `-e` — Display acl in a table (unstable)

  Default value: `false`
* `--private` — Display private chunks in a table (unstable)

  Default value: `false`
* `--numeric-owner` — Display user id and group id instead of user name and group name

  Default value: `false`
* `-T` — When used with the -l option, display complete time information for the entry, including month, day, hour, minute, second, and year

  Default value: `false`
* `--format <FORMAT>` — Display format [unstable: jsonl, bsdtar, csv, tsv]

  Possible values: `line`, `table`, `jsonl`, `tree`, `bsdtar`, `csv`, `tsv`

* `--time <TIME>` — Which timestamp field to list (modified, accessed, created)

  Possible values: `created`, `modified`, `accessed`

* `-q` — Force printing of non-graphic characters in file names as the character '?'

  Default value: `false`
* `--classify` — Display type indicator by entry kinds

  Default value: `false`
* `--include <INCLUDE>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions
* `--exclude <EXCLUDE>` — Exclude path glob (unstable)
* `--exclude-from <EXCLUDE_FROM>` — Read exclude files from given path (unstable)
* `--exclude-vcs` — Exclude vcs files (unstable)

  Default value: `false`
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `-f`, `--file <FILE>`
* `--help` — Print help
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`



## `pna split`

Split archive

**Usage:** `pna split [OPTIONS] <--file <FILE>|ARCHIVE>`

###### **Arguments:**

* `<ARCHIVE>`

###### **Options:**

* `-f`, `--file <FILE>`
* `--out-dir <OUT_DIR>`
* `--overwrite` — Overwrite file

  Default value: `false`
* `--no-overwrite` — Do not overwrite files. This is the inverse option of --overwrite

  Default value: `false`
* `--max-size <size>` — Maximum size in bytes of split archive (minimum 64B)
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna concat`

Concat archives

**Usage:** `pna concat [OPTIONS] <--files <FILES>|ARCHIVES>`

###### **Arguments:**

* `<ARCHIVES>`

###### **Options:**

* `--overwrite` — Overwrite file

  Default value: `false`
* `--no-overwrite` — Do not overwrite files. This is the inverse option of --overwrite

  Default value: `false`
* `-f`, `--files <FILES>`
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna strip`

Strip entries metadata

**Usage:** `pna strip [OPTIONS] <--file <FILE>|ARCHIVE> [FILES]...`

###### **Arguments:**

* `<ARCHIVE>`
* `<FILES>`

###### **Options:**

* `--keep-timestamp` [alias: `preserve-timestamps`] — Keep the timestamp of the files

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Keep the permissions of the files

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Keep the extended attributes of the files

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Keep the acl of the files

  Default value: `false`
* `--keep-private <KEEP_PRIVATE>` [alias: `preserve-private_chunks`] — Keep private chunks
* `--unsolid` — Unsolid input solid entries.

  Default value: `false`
* `--keep-solid` — Keep input solid entries.

  Default value: `false`
* `--output <OUTPUT>` — Output file path
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `-f`, `--file <FILE>`
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna sort`

Sort entries in archive

**Usage:** `pna sort [OPTIONS] --file <ARCHIVE>`

###### **Options:**

* `-f`, `--file <ARCHIVE>`
* `--output <OUTPUT>` — Output file path
* `--by <KEY>` — Sort key in format KEY[:ORDER] (e.g., name, mtime:desc) [keys: name, ctime, mtime, atime] [orders: asc, desc]

  Default value: `name`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna xattr`

Manipulate extended attributes

**Usage:** `pna xattr [OPTIONS]
       pna xattr <COMMAND>`

###### **Subcommands:**

* `get` — Get extended attributes of entries
* `set` — Set extended attributes of entries
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna xattr get`

Get extended attributes of entries

**Usage:** `pna xattr get [OPTIONS] <--file <FILE>|ARCHIVE> [FILES]...`

###### **Arguments:**

* `<ARCHIVE>`
* `<FILES>`

###### **Options:**

* `-f`, `--file <FILE>`
* `-n`, `--name <NAME>` — Dump the value of the named extended attribute
* `-d`, `--dump` — Dump the values of all matched extended attributes

  Default value: `false`
* `-m`, `--match <pattern>` — Only include attributes with names matching the regular expression pattern. Specify '-' for including all attributes
* `-e`, `--encoding <ENCODING>` — Encode values after retrieving them

  Possible values: `text`, `hex`, `base64`

* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna xattr set`

Set extended attributes of entries

**Usage:** `pna xattr set [OPTIONS] <--file <FILE>|ARCHIVE> [FILES]...`

###### **Arguments:**

* `<ARCHIVE>`
* `<FILES>`

###### **Options:**

* `-f`, `--file <FILE>`
* `-n`, `--name <NAME>` — Name of extended attribute
* `-v`, `--value <VALUE>` — Value of extended attribute
* `-x`, `--remove <REMOVE>` — Remove extended attribute
* `--restore <RESTORE>` — Restores extended attributes from file. The file must be in the format generated by the pna xattr get command with the --dump option. If a dash (-) is given as the file name, reads from standard input
* `--unsolid` — Unsolid input solid entries.

  Default value: `false`
* `--keep-solid` — Keep input solid entries.

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna xattr help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna xattr help [COMMAND]`

###### **Subcommands:**

* `get` — Get extended attributes of entries
* `set` — Set extended attributes of entries
* `help` — Print this message or the help of the given subcommand(s)



## `pna xattr help get`

Get extended attributes of entries

**Usage:** `pna xattr help get`



## `pna xattr help set`

Set extended attributes of entries

**Usage:** `pna xattr help set`



## `pna xattr help help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna xattr help help`



## `pna complete`

Generate shell auto complete

**Usage:** `pna complete [OPTIONS] <SHELL>`

###### **Arguments:**

* `<SHELL>` — shell

  Possible values: `bash`, `elvish`, `fish`, `powershell`, `zsh`


###### **Options:**

* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna bug-report`

Generate bug report template

**Usage:** `pna bug-report [OPTIONS]`

###### **Options:**

* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental`

Unstable experimental commands; behavior and interface may change or be removed

**Usage:** `pna experimental [OPTIONS]
       pna experimental <COMMAND>`

###### **Subcommands:**

* `stdio` — Archive manipulation via stdio
* `delete` — Delete entry from archive
* `update` — Update entries in archive
* `chown` — Change owner
* `chmod` — Change mode
* `xattr` — Manipulate extended attributes (stabilized, use `pna xattr` command instead. this command will be removed in the future)
* `acl` — Manipulate ACLs of entries
* `migrate` — Migrate old format to latest format
* `chunk` — Chunk level operation
* `sort` — Sort entries in archive (stabilized, use `pna sort` command instead. this command will be removed in the future)
* `diff` — Compare archive entries with filesystem
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental stdio`

Archive manipulation via stdio

**Usage:** `pna experimental stdio [OPTIONS] <--create|--extract|--list|--append> [FILES]...`

###### **Arguments:**

* `<FILES>` — Files or patterns

###### **Options:**

* `--one-file-system` — Stay in the same file system when collecting files (unstable)

  Default value: `false`
* `--nodump` — Exclude files with the nodump flag (unstable)

  Default value: `false`
* `-c`, `--create` — Create archive

  Default value: `false`
* `-x`, `--extract` — Extract archive

  Default value: `false`
* `-t`, `--list` — List files in archive

  Default value: `false`
* `--append` — Append files to archive

  Default value: `false`
* `--recursive` [alias: `recursion`] — Add directories to the archive recursively

  Default value: `true`
* `-n`, `--no-recursive` [aliases: `norecurse`, `no-recursion`] — Do not recursively add directories to the archives. This is the inverse option of --recursive

  Default value: `false`
* `--overwrite` — Overwrite file

  Default value: `false`
* `--no-overwrite` — Do not overwrite files. This is the inverse option of --overwrite

  Default value: `false`
* `--keep-newer-files` — Skip extracting files if a newer version already exists

  Default value: `false`
* `-U`, `--unlink-first` [alias: `unlink`] — Unlink files before creating them; also removes intervening directory symlinks (extract mode only)

  Default value: `false`
* `-k`, `--keep-old-files` — Skip extracting files if they already exist

  Default value: `false`
* `--keep-dir` — Archiving the directories

  Default value: `false`
* `--no-keep-dir` — Do not archive directories. This is the inverse option of --keep-dir

  Default value: `false`
* `--keep-timestamp` [alias: `preserve-timestamps`] — Archiving the timestamp of the files

  Default value: `false`
* `-m`, `--no-keep-timestamp` [aliases: `no-preserve-timestamps`, `modification_time`] — Do not archive timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Archiving the permissions of the files (unstable on Windows)

  Default value: `false`
* `--no-keep-permission` [aliases: `no-preserve-permissions`, `no-permissions`] — Do not archive permissions of files. This is the inverse option of --preserve-permissions

  Default value: `false`
* `--keep-xattr` [aliases: `preserve-xattrs`, `xattrs`] — Archiving the extended attributes of the files

  Default value: `false`
* `--no-keep-xattr` [aliases: `no-preserve-xattrs`, `no-xattrs`] — Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [aliases: `preserve-acls`, `acls`] — Archiving the acl of the files (unstable)

  Default value: `false`
* `--no-keep-acl` [aliases: `no-preserve-acls`, `no-acls`] — Do not archive acl of files. This is the inverse option of --keep-acl (unstable)

  Default value: `false`
* `--solid` — Solid mode archive

  Default value: `false`
* `--store` — No compression

  Default value: `false`
* `--deflate <level>` — Use deflate for compression [possible level: 1-9, min, max]
* `--zstd <level>` — Use zstd for compression [possible level: 1-21, min, max]
* `--xz <level>` — Use xz for compression [possible level: 0-9, min, max]
* `--aes <cipher mode>` — Use aes for encryption

  Possible values: `cbc`, `ctr`

* `--camellia <cipher mode>` — Use camellia for encryption

  Possible values: `cbc`, `ctr`

* `--argon2 <ARGON2>` — Use argon2 for password hashing
* `--pbkdf2 <PBKDF2>` — Use pbkdf2 for password hashing
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--include <INCLUDE>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)
* `--exclude <EXCLUDE>` — Exclude path glob (unstable)
* `-X`, `--exclude-from <EXCLUDE_FROM>` — Read exclude files from given path (unstable)
* `--exclude-vcs` — Exclude vcs files (unstable)

  Default value: `false`
* `--gitignore` — Ignore files from .gitignore (unstable)

  Default value: `false`
* `-L`, `--follow-links` [alias: `dereference`] — Follow symbolic links

  Default value: `false`
* `-H`, `--follow-command-links` — Follow symbolic links named on the command line

  Default value: `false`
* `--out-dir <OUT_DIR>` — Output directory of extracted files
* `--strip-components <STRIP_COMPONENTS>` — Remove the specified number of leading path elements. Path names with fewer elements will be silently skipped
* `--uname <UNAME>` — On create, archiving user to the entries from given name. On extract, restore user from given name
* `--gname <GNAME>` — On create, archiving group to the entries from given name. On extract, restore group from given name
* `--uid <UID>` — On create, this overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id. On extract, this overrides the user id in the archive; the user name in the archive will be ignored
* `--gid <GID>` — On create, this overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id. On extract, this overrides the group id in the archive; the group name in the archive will be ignored
* `--numeric-owner` — This is equivalent to --uname "" --gname "". On create, it causes user and group names to not be stored in the archive. On extract, it causes user and group names in the archive to be ignored in favor of the numeric user and group ids.

  Default value: `false`
* `--ctime <CTIME>` — Overrides the creation time
* `--clamp-ctime` — Clamp the creation time of the entries to the specified time by --ctime

  Default value: `false`
* `--atime <ATIME>` — Overrides the access time
* `--clamp-atime` — Clamp the access time of the entries to the specified time by --atime

  Default value: `false`
* `--mtime <MTIME>` — Overrides the modification time
* `--clamp-mtime` — Clamp the modification time of the entries to the specified time by --mtime

  Default value: `false`
* `--older-ctime <OLDER_CTIME>` — Only include files and directories older than the specified date (unstable). This compares ctime entries.
* `--older-mtime <OLDER_MTIME>` — Only include files and directories older than the specified date (unstable). This compares mtime entries.
* `--newer-ctime <NEWER_CTIME>` — Only include files and directories newer than the specified date (unstable). This compares ctime entries.
* `--newer-mtime <NEWER_MTIME>` — Only include files and directories newer than the specified date (unstable). This compares mtime entries.
* `--newer-ctime-than <file>` — Only include files and directories newer than the specified file (unstable). This compares ctime entries.
* `--newer-mtime-than <file>` [alias: `newer-than`] — Only include files and directories newer than the specified file (unstable). This compares mtime entries.
* `--older-ctime-than <file>` — Only include files and directories older than the specified file (unstable). This compares ctime entries.
* `--older-mtime-than <file>` [alias: `older-than`] — Only include files and directories older than the specified file (unstable). This compares mtime entries.
* `-T`, `--files-from <FILES_FROM>` — Read archiving files from given path (unstable)
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option (unstable)
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option (unstable)
* `--same-owner` — Try extracting files with the same ownership as exists in the archive

  Default value: `false`
* `--no-same-owner` — Extract files as yourself

  Default value: `false`
* `-C`, `--cd <DIRECTORY>` [alias: `directory`] — changes the directory before adding the following files
* `-O`, `--to-stdout` — Write extracted file data to standard output instead of the file system

  Default value: `false`
* `--allow-unsafe-links` — Allow extracting symbolic links and hard links that contain root or parent paths

  Default value: `false`
* `-P`, `--absolute-paths` — Do not strip leading '/' or '..' from member names and link targets (unstable)

  Default value: `false`
* `-f`, `--file <FILE>` — Read the archive from or write the archive to the specified file. The filename can be - for standard input or standard output.
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `-v` — Verbose

  Default value: `false`
* `--version` — Print version
* `--help` — Print help
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`



## `pna experimental delete`

Delete entry from archive

**Usage:** `pna experimental delete [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>`

###### **Options:**

* `--output <OUTPUT>` — Output file path
* `--files-from <FILES_FROM>` — Read deleting files from given path (unstable)
* `--files-from-stdin` — Read deleting files from stdin (unstable)

  Default value: `false`
* `--include <INCLUDE>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)
* `--exclude <EXCLUDE>` — Exclude path glob (unstable)
* `--exclude-from <EXCLUDE_FROM>` — Read exclude files from given path (unstable)
* `--exclude-vcs` — Exclude vcs files (unstable)

  Default value: `false`
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--unsolid` — Unsolid input solid entries.

  Default value: `false`
* `--keep-solid` — Keep input solid entries.

  Default value: `false`
* `-f`, `--file <ARCHIVE>`
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental update`

Update entries in archive

**Usage:** `pna experimental update [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>`

###### **Options:**

* `--one-file-system` — Stay in the same file system when collecting files (unstable)

  Default value: `false`
* `--nodump` — Exclude files with the nodump flag (unstable)

  Default value: `false`
* `-r`, `--recursive` [alias: `recursion`] — Add the directory to the archive recursively

  Default value: `true`
* `--no-recursive` [alias: `no-recursion`] — Do not recursively add directories to the archives. This is the inverse option of --recursive

  Default value: `false`
* `--keep-dir` — Archiving the directories

  Default value: `false`
* `--no-keep-dir` — Do not archive directories. This is the inverse option of --keep-dir

  Default value: `false`
* `--keep-timestamp` [alias: `preserve-timestamps`] — Archiving the timestamp of the files

  Default value: `false`
* `--no-keep-timestamp` [alias: `no-preserve-timestamps`] — Do not archive timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Archiving the permissions of the files (unstable on Windows)

  Default value: `false`
* `--no-keep-permission` [alias: `no-preserve-permissions`] — Do not archive permissions of files. This is the inverse option of --preserve-permissions

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Archiving the extended attributes of the files

  Default value: `false`
* `--no-keep-xattr` [alias: `no-preserve-xattrs`] — Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Archiving the acl of the files (unstable)

  Default value: `false`
* `--no-keep-acl` [alias: `no-preserve-acls`] — Do not archive acl of files. This is the inverse option of --keep-acl (unstable)

  Default value: `false`
* `--uname <UNAME>` — Archiving user to the entries from given name
* `--gname <GNAME>` — Archiving group to the entries from given name
* `--uid <UID>` — Overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id
* `--gid <GID>` — Overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id
* `--strip-components <STRIP_COMPONENTS>` — Remove the specified number of leading path elements when storing paths (unstable)
* `--numeric-owner` — This is equivalent to --uname "" --gname "". It causes user and group names to not be stored in the archive

  Default value: `false`
* `--ctime <CTIME>` — Overrides the creation time read from disk
* `--clamp-ctime` — Clamp the creation time of the entries to the specified time by --ctime

  Default value: `false`
* `--atime <ATIME>` — Overrides the access time read from disk
* `--clamp-atime` — Clamp the access time of the entries to the specified time by --atime

  Default value: `false`
* `--mtime <MTIME>` — Overrides the modification time read from disk
* `--clamp-mtime` — Clamp the modification time of the entries to the specified time by --mtime

  Default value: `false`
* `--older-ctime <OLDER_CTIME>` — Only include files and directories older than the specified date. This compares ctime entries.
* `--older-mtime <OLDER_MTIME>` — Only include files and directories older than the specified date. This compares mtime entries.
* `--newer-ctime <NEWER_CTIME>` — Only include files and directories newer than the specified date. This compares ctime entries.
* `--newer-mtime <NEWER_MTIME>` — Only include files and directories newer than the specified date. This compares mtime entries.
* `--newer-ctime-than <file>` — Only include files and directories newer than the specified file (unstable). This compares ctime entries.
* `--newer-mtime-than <file>` — Only include files and directories newer than the specified file (unstable). This compares mtime entries.
* `--older-ctime-than <file>` — Only include files and directories older than the specified file (unstable). This compares ctime entries.
* `--older-mtime-than <file>` — Only include files and directories older than the specified file (unstable). This compares mtime entries.
* `--files-from <FILES_FROM>` — Read archiving files from given path (unstable)
* `--files-from-stdin` — Read archiving files from stdin (unstable)

  Default value: `false`
* `--include <INCLUDE>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions (unstable)
* `--exclude <EXCLUDE>` — Exclude path glob (unstable)
* `--exclude-from <EXCLUDE_FROM>` — Read exclude files from given path (unstable)
* `--exclude-vcs` — Exclude vcs files (unstable)

  Default value: `false`
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option (unstable)
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option (unstable)
* `-C`, `--cd <DIRECTORY>` [alias: `directory`] — changes the directory before adding the following files
* `--store` — No compression

  Default value: `false`
* `--deflate <level>` — Use deflate for compression [possible level: 1-9, min, max]
* `--zstd <level>` — Use zstd for compression [possible level: 1-21, min, max]
* `--xz <level>` — Use xz for compression [possible level: 0-9, min, max]
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--aes <cipher mode>` — Use aes for encryption

  Possible values: `cbc`, `ctr`

* `--camellia <cipher mode>` — Use camellia for encryption

  Possible values: `cbc`, `ctr`

* `--argon2 <ARGON2>` — Use argon2 for password hashing
* `--pbkdf2 <PBKDF2>` — Use pbkdf2 for password hashing
* `--unsolid` — Unsolid input solid entries.

  Default value: `false`
* `--keep-solid` — Keep input solid entries.

  Default value: `false`
* `-f`, `--file <ARCHIVE>`
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `--gitignore` — Ignore files from .gitignore (unstable)

  Default value: `false`
* `--follow-links` [alias: `dereference`] — Follow symbolic links

  Default value: `false`
* `-H`, `--follow-command-links` — Follow symbolic links named on the command line

  Default value: `false`
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental chown`

Change owner

**Usage:** `pna experimental chown [OPTIONS] --file <ARCHIVE> <OWNER> [FILES]...`

###### **Arguments:**

* `<OWNER>` — owner[:group]|:group
* `<FILES>`

###### **Options:**

* `-f`, `--file <ARCHIVE>`
* `--numeric-owner` — force numeric owner and group IDs (no name resolution)

  Default value: `false`
* `--owner-lookup` — resolve user and group (default)

  Default value: `false`
* `--no-owner-lookup` — do not resolve user and group

  Default value: `false`
* `--unsolid` — Unsolid input solid entries.

  Default value: `false`
* `--keep-solid` — Keep input solid entries.

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental chmod`

Change mode

**Usage:** `pna experimental chmod [OPTIONS] --file <ARCHIVE> <MODE> [FILES]...`

###### **Arguments:**

* `<MODE>` — mode
* `<FILES>`

###### **Options:**

* `-f`, `--file <ARCHIVE>`
* `--unsolid` — Unsolid input solid entries.

  Default value: `false`
* `--keep-solid` — Keep input solid entries.

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental xattr`

Manipulate extended attributes (stabilized, use `pna xattr` command instead. this command will be removed in the future)

**Usage:** `pna experimental xattr [OPTIONS]
       pna experimental xattr <COMMAND>`

###### **Subcommands:**

* `get` — Get extended attributes of entries
* `set` — Set extended attributes of entries
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental xattr get`

Get extended attributes of entries

**Usage:** `pna experimental xattr get [OPTIONS] <--file <FILE>|ARCHIVE> [FILES]...`

###### **Arguments:**

* `<ARCHIVE>`
* `<FILES>`

###### **Options:**

* `-f`, `--file <FILE>`
* `-n`, `--name <NAME>` — Dump the value of the named extended attribute
* `-d`, `--dump` — Dump the values of all matched extended attributes

  Default value: `false`
* `-m`, `--match <pattern>` — Only include attributes with names matching the regular expression pattern. Specify '-' for including all attributes
* `-e`, `--encoding <ENCODING>` — Encode values after retrieving them

  Possible values: `text`, `hex`, `base64`

* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental xattr set`

Set extended attributes of entries

**Usage:** `pna experimental xattr set [OPTIONS] <--file <FILE>|ARCHIVE> [FILES]...`

###### **Arguments:**

* `<ARCHIVE>`
* `<FILES>`

###### **Options:**

* `-f`, `--file <FILE>`
* `-n`, `--name <NAME>` — Name of extended attribute
* `-v`, `--value <VALUE>` — Value of extended attribute
* `-x`, `--remove <REMOVE>` — Remove extended attribute
* `--restore <RESTORE>` — Restores extended attributes from file. The file must be in the format generated by the pna xattr get command with the --dump option. If a dash (-) is given as the file name, reads from standard input
* `--unsolid` — Unsolid input solid entries.

  Default value: `false`
* `--keep-solid` — Keep input solid entries.

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental xattr help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna experimental xattr help [COMMAND]`

###### **Subcommands:**

* `get` — Get extended attributes of entries
* `set` — Set extended attributes of entries
* `help` — Print this message or the help of the given subcommand(s)



## `pna experimental xattr help get`

Get extended attributes of entries

**Usage:** `pna experimental xattr help get`



## `pna experimental xattr help set`

Set extended attributes of entries

**Usage:** `pna experimental xattr help set`



## `pna experimental xattr help help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna experimental xattr help help`



## `pna experimental acl`

Manipulate ACLs of entries

**Usage:** `pna experimental acl [OPTIONS]
       pna experimental acl <COMMAND>`

###### **Subcommands:**

* `get` — Get acl of entries
* `set` — Set acl of entries
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental acl get`

Get acl of entries

**Usage:** `pna experimental acl get [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>`

###### **Options:**

* `--platform <PLATFORM>` — Display specified ACL platform
* `-n`, `--numeric` — List numeric user and group IDs

  Default value: `false`
* `-f`, `--file <ARCHIVE>`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental acl set`

Set acl of entries

**Usage:** `pna experimental acl set [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>`

###### **Options:**

* `-f`, `--file <ARCHIVE>`
* `--set <SET>` — Set the ACL on the specified file.
* `-m`, `--modify <MODIFY>` — Modify the ACL on the specified file. New entries will be added, and existing entries will be modified according to the entries argument.
* `-x`, `--remove <REMOVE>` — Remove the ACL entries specified there from the access or default ACL of the specified files.
* `--platform <PLATFORM>` — Target ACL platform

  Default value: ``
* `--restore <RESTORE>` — Restore a permission backup created by `pna acl get *` or similar. All permissions of a complete directory subtree are restored using this mechanism. If a dash (-) is given as the file name, reads from standard input
* `--unsolid` — Unsolid input solid entries.

  Default value: `false`
* `--keep-solid` — Keep input solid entries.

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental acl help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna experimental acl help [COMMAND]`

###### **Subcommands:**

* `get` — Get acl of entries
* `set` — Set acl of entries
* `help` — Print this message or the help of the given subcommand(s)



## `pna experimental acl help get`

Get acl of entries

**Usage:** `pna experimental acl help get`



## `pna experimental acl help set`

Set acl of entries

**Usage:** `pna experimental acl help set`



## `pna experimental acl help help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna experimental acl help help`



## `pna experimental migrate`

Migrate old format to latest format

**Usage:** `pna experimental migrate [OPTIONS] --file <ARCHIVE> --output <OUTPUT>`

###### **Options:**

* `--unsolid` — Unsolid input solid entries.

  Default value: `false`
* `--keep-solid` — Keep input solid entries.

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `-f`, `--file <ARCHIVE>`
* `--output <OUTPUT>` — Output file path
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental chunk`

Chunk level operation

**Usage:** `pna experimental chunk [OPTIONS]
       pna experimental chunk <COMMAND>`

###### **Subcommands:**

* `list` — List chunks
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental chunk list`

List chunks

**Usage:** `pna experimental chunk list [OPTIONS] --file <ARCHIVE>`

###### **Options:**

* `-l`, `--long` — Display chunk body

  Default value: `false`
* `-h`, `--header` — Add a header row to each column

  Default value: `false`
* `-f`, `--file <ARCHIVE>`
* `--help` — Print help
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`



## `pna experimental chunk help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna experimental chunk help [COMMAND]`

###### **Subcommands:**

* `list` — List chunks
* `help` — Print this message or the help of the given subcommand(s)



## `pna experimental chunk help list`

List chunks

**Usage:** `pna experimental chunk help list`



## `pna experimental chunk help help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna experimental chunk help help`



## `pna experimental sort`

Sort entries in archive (stabilized, use `pna sort` command instead. this command will be removed in the future)

**Usage:** `pna experimental sort [OPTIONS] --file <ARCHIVE>`

###### **Options:**

* `-f`, `--file <ARCHIVE>`
* `--output <OUTPUT>` — Output file path
* `--by <KEY>` — Sort key in format KEY[:ORDER] (e.g., name, mtime:desc) [keys: name, ctime, mtime, atime] [orders: asc, desc]

  Default value: `name`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental diff`

Compare archive entries with filesystem

**Usage:** `pna experimental diff [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>`

###### **Options:**

* `-f`, `--file <ARCHIVE>`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <PASSWORD_FILE>` — Read password from specified file
* `--quiet` — Make some output more quiet

  Default value: `false`
* `--verbose` — Make some output more verbose

  Default value: `false`
* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna experimental help [COMMAND]`

###### **Subcommands:**

* `stdio` — Archive manipulation via stdio
* `delete` — Delete entry from archive
* `update` — Update entries in archive
* `chown` — Change owner
* `chmod` — Change mode
* `xattr` — Manipulate extended attributes (stabilized, use `pna xattr` command instead. this command will be removed in the future)
* `acl` — Manipulate ACLs of entries
* `migrate` — Migrate old format to latest format
* `chunk` — Chunk level operation
* `sort` — Sort entries in archive (stabilized, use `pna sort` command instead. this command will be removed in the future)
* `diff` — Compare archive entries with filesystem
* `help` — Print this message or the help of the given subcommand(s)



## `pna experimental help stdio`

Archive manipulation via stdio

**Usage:** `pna experimental help stdio`



## `pna experimental help delete`

Delete entry from archive

**Usage:** `pna experimental help delete`



## `pna experimental help update`

Update entries in archive

**Usage:** `pna experimental help update`



## `pna experimental help chown`

Change owner

**Usage:** `pna experimental help chown`



## `pna experimental help chmod`

Change mode

**Usage:** `pna experimental help chmod`



## `pna experimental help xattr`

Manipulate extended attributes (stabilized, use `pna xattr` command instead. this command will be removed in the future)

**Usage:** `pna experimental help xattr [COMMAND]`

###### **Subcommands:**

* `get` — Get extended attributes of entries
* `set` — Set extended attributes of entries



## `pna experimental help xattr get`

Get extended attributes of entries

**Usage:** `pna experimental help xattr get`



## `pna experimental help xattr set`

Set extended attributes of entries

**Usage:** `pna experimental help xattr set`



## `pna experimental help acl`

Manipulate ACLs of entries

**Usage:** `pna experimental help acl [COMMAND]`

###### **Subcommands:**

* `get` — Get acl of entries
* `set` — Set acl of entries



## `pna experimental help acl get`

Get acl of entries

**Usage:** `pna experimental help acl get`



## `pna experimental help acl set`

Set acl of entries

**Usage:** `pna experimental help acl set`



## `pna experimental help migrate`

Migrate old format to latest format

**Usage:** `pna experimental help migrate`



## `pna experimental help chunk`

Chunk level operation

**Usage:** `pna experimental help chunk [COMMAND]`

###### **Subcommands:**

* `list` — List chunks



## `pna experimental help chunk list`

List chunks

**Usage:** `pna experimental help chunk list`



## `pna experimental help sort`

Sort entries in archive (stabilized, use `pna sort` command instead. this command will be removed in the future)

**Usage:** `pna experimental help sort`



## `pna experimental help diff`

Compare archive entries with filesystem

**Usage:** `pna experimental help diff`



## `pna experimental help help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna experimental help help`



## `pna help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna help [COMMAND]`

###### **Subcommands:**

* `create` — Create archive
* `append` — Append files to archive
* `extract` — Extract files from archive
* `list` — List files in archive
* `split` — Split archive
* `concat` — Concat archives
* `strip` — Strip entries metadata
* `sort` — Sort entries in archive
* `xattr` — Manipulate extended attributes
* `complete` — Generate shell auto complete
* `bug-report` — Generate bug report template
* `experimental` — Unstable experimental commands; behavior and interface may change or be removed
* `help` — Print this message or the help of the given subcommand(s)



## `pna help create`

Create archive

**Usage:** `pna help create`



## `pna help append`

Append files to archive

**Usage:** `pna help append`



## `pna help extract`

Extract files from archive

**Usage:** `pna help extract`



## `pna help list`

List files in archive

**Usage:** `pna help list`



## `pna help split`

Split archive

**Usage:** `pna help split`



## `pna help concat`

Concat archives

**Usage:** `pna help concat`



## `pna help strip`

Strip entries metadata

**Usage:** `pna help strip`



## `pna help sort`

Sort entries in archive

**Usage:** `pna help sort`



## `pna help xattr`

Manipulate extended attributes

**Usage:** `pna help xattr [COMMAND]`

###### **Subcommands:**

* `get` — Get extended attributes of entries
* `set` — Set extended attributes of entries



## `pna help xattr get`

Get extended attributes of entries

**Usage:** `pna help xattr get`



## `pna help xattr set`

Set extended attributes of entries

**Usage:** `pna help xattr set`



## `pna help complete`

Generate shell auto complete

**Usage:** `pna help complete`



## `pna help bug-report`

Generate bug report template

**Usage:** `pna help bug-report`



## `pna help experimental`

Unstable experimental commands; behavior and interface may change or be removed

**Usage:** `pna help experimental [COMMAND]`

###### **Subcommands:**

* `stdio` — Archive manipulation via stdio
* `delete` — Delete entry from archive
* `update` — Update entries in archive
* `chown` — Change owner
* `chmod` — Change mode
* `xattr` — Manipulate extended attributes (stabilized, use `pna xattr` command instead. this command will be removed in the future)
* `acl` — Manipulate ACLs of entries
* `migrate` — Migrate old format to latest format
* `chunk` — Chunk level operation
* `sort` — Sort entries in archive (stabilized, use `pna sort` command instead. this command will be removed in the future)
* `diff` — Compare archive entries with filesystem



## `pna help experimental stdio`

Archive manipulation via stdio

**Usage:** `pna help experimental stdio`



## `pna help experimental delete`

Delete entry from archive

**Usage:** `pna help experimental delete`



## `pna help experimental update`

Update entries in archive

**Usage:** `pna help experimental update`



## `pna help experimental chown`

Change owner

**Usage:** `pna help experimental chown`



## `pna help experimental chmod`

Change mode

**Usage:** `pna help experimental chmod`



## `pna help experimental xattr`

Manipulate extended attributes (stabilized, use `pna xattr` command instead. this command will be removed in the future)

**Usage:** `pna help experimental xattr [COMMAND]`

###### **Subcommands:**

* `get` — Get extended attributes of entries
* `set` — Set extended attributes of entries



## `pna help experimental xattr get`

Get extended attributes of entries

**Usage:** `pna help experimental xattr get`



## `pna help experimental xattr set`

Set extended attributes of entries

**Usage:** `pna help experimental xattr set`



## `pna help experimental acl`

Manipulate ACLs of entries

**Usage:** `pna help experimental acl [COMMAND]`

###### **Subcommands:**

* `get` — Get acl of entries
* `set` — Set acl of entries



## `pna help experimental acl get`

Get acl of entries

**Usage:** `pna help experimental acl get`



## `pna help experimental acl set`

Set acl of entries

**Usage:** `pna help experimental acl set`



## `pna help experimental migrate`

Migrate old format to latest format

**Usage:** `pna help experimental migrate`



## `pna help experimental chunk`

Chunk level operation

**Usage:** `pna help experimental chunk [COMMAND]`

###### **Subcommands:**

* `list` — List chunks



## `pna help experimental chunk list`

List chunks

**Usage:** `pna help experimental chunk list`



## `pna help experimental sort`

Sort entries in archive (stabilized, use `pna sort` command instead. this command will be removed in the future)

**Usage:** `pna help experimental sort`



## `pna help experimental diff`

Compare archive entries with filesystem

**Usage:** `pna help experimental diff`



## `pna help help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna help help`



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
