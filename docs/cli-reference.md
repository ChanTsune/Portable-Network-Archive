# Command-Line Help for `pna`

This document contains the help content for the `pna` command-line program.

**Command Overview:**

* [`pna`↴](#pna)
* [`pna create`↴](#pna-create)
* [`pna append`↴](#pna-append)
* [`pna extract`↴](#pna-extract)
* [`pna list`↴](#pna-list)
* [`pna delete`↴](#pna-delete)
* [`pna split`↴](#pna-split)
* [`pna concat`↴](#pna-concat)
* [`pna strip`↴](#pna-strip)
* [`pna sort`↴](#pna-sort)
* [`pna migrate`↴](#pna-migrate)
* [`pna xattr`↴](#pna-xattr)
* [`pna xattr get`↴](#pna-xattr-get)
* [`pna xattr set`↴](#pna-xattr-set)
* [`pna xattr help`↴](#pna-xattr-help)
* [`pna xattr help get`↴](#pna-xattr-help-get)
* [`pna xattr help set`↴](#pna-xattr-help-set)
* [`pna xattr help help`↴](#pna-xattr-help-help)
* [`pna complete`↴](#pna-complete)
* [`pna bug-report`↴](#pna-bug-report)
* [`pna compat`↴](#pna-compat)
* [`pna compat bsdtar`↴](#pna-compat-bsdtar)
* [`pna compat help`↴](#pna-compat-help)
* [`pna compat help bsdtar`↴](#pna-compat-help-bsdtar)
* [`pna compat help help`↴](#pna-compat-help-help)
* [`pna experimental`↴](#pna-experimental)
* [`pna experimental update`↴](#pna-experimental-update)
* [`pna experimental chown`↴](#pna-experimental-chown)
* [`pna experimental chmod`↴](#pna-experimental-chmod)
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
* [`pna experimental diff`↴](#pna-experimental-diff)
* [`pna experimental verify`↴](#pna-experimental-verify)
* [`pna experimental help`↴](#pna-experimental-help)
* [`pna experimental help update`↴](#pna-experimental-help-update)
* [`pna experimental help chown`↴](#pna-experimental-help-chown)
* [`pna experimental help chmod`↴](#pna-experimental-help-chmod)
* [`pna experimental help acl`↴](#pna-experimental-help-acl)
* [`pna experimental help acl get`↴](#pna-experimental-help-acl-get)
* [`pna experimental help acl set`↴](#pna-experimental-help-acl-set)
* [`pna experimental help migrate`↴](#pna-experimental-help-migrate)
* [`pna experimental help chunk`↴](#pna-experimental-help-chunk)
* [`pna experimental help chunk list`↴](#pna-experimental-help-chunk-list)
* [`pna experimental help diff`↴](#pna-experimental-help-diff)
* [`pna experimental help verify`↴](#pna-experimental-help-verify)
* [`pna experimental help help`↴](#pna-experimental-help-help)
* [`pna help`↴](#pna-help)
* [`pna help create`↴](#pna-help-create)
* [`pna help append`↴](#pna-help-append)
* [`pna help extract`↴](#pna-help-extract)
* [`pna help list`↴](#pna-help-list)
* [`pna help delete`↴](#pna-help-delete)
* [`pna help split`↴](#pna-help-split)
* [`pna help concat`↴](#pna-help-concat)
* [`pna help strip`↴](#pna-help-strip)
* [`pna help sort`↴](#pna-help-sort)
* [`pna help migrate`↴](#pna-help-migrate)
* [`pna help xattr`↴](#pna-help-xattr)
* [`pna help xattr get`↴](#pna-help-xattr-get)
* [`pna help xattr set`↴](#pna-help-xattr-set)
* [`pna help complete`↴](#pna-help-complete)
* [`pna help bug-report`↴](#pna-help-bug-report)
* [`pna help compat`↴](#pna-help-compat)
* [`pna help compat bsdtar`↴](#pna-help-compat-bsdtar)
* [`pna help experimental`↴](#pna-help-experimental)
* [`pna help experimental update`↴](#pna-help-experimental-update)
* [`pna help experimental chown`↴](#pna-help-experimental-chown)
* [`pna help experimental chmod`↴](#pna-help-experimental-chmod)
* [`pna help experimental acl`↴](#pna-help-experimental-acl)
* [`pna help experimental acl get`↴](#pna-help-experimental-acl-get)
* [`pna help experimental acl set`↴](#pna-help-experimental-acl-set)
* [`pna help experimental migrate`↴](#pna-help-experimental-migrate)
* [`pna help experimental chunk`↴](#pna-help-experimental-chunk)
* [`pna help experimental chunk list`↴](#pna-help-experimental-chunk-list)
* [`pna help experimental diff`↴](#pna-help-experimental-diff)
* [`pna help experimental verify`↴](#pna-help-experimental-verify)
* [`pna help help`↴](#pna-help-help)

## `pna`

Portable-Network-Archive cli

**Usage:** `pna [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `create` — Create archive
* `append` — Append files to archive
* `extract` — Extract files from archive
* `list` — List files in archive
* `delete` — Delete entry from archive
* `split` — Split archive
* `concat` — Concat archives
* `strip` — Strip entries metadata
* `sort` — Sort entries in archive
* `migrate` — Upgrade archives created by older PNA versions
* `xattr` — Manipulate extended attributes
* `complete` — Generate shell auto complete
* `bug-report` — Generate bug report template
* `compat` — Compatibility interface for other archive tools
* `experimental` — Unstable experimental commands; behavior and interface may change or be removed
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help
* `-V`, `--version` — Print version



## `pna create`

Create archive

**Usage:** `pna create [OPTIONS] --file <ARCHIVE> [FILES]...`

**Command Alias:** `c`

###### **Arguments:**

* `<FILES>` — Files or directories to process

###### **Options:**

* `--one-file-system` — Stay in the same file system when collecting files

  Default value: `false`
* `--nodump` — Exclude files with the nodump flag

  Default value: `false`
* `-r`, `--recursive` [alias: `recursion`] — Add the directory to the archive recursively

  Default value: `true`
* `--no-recursive` [alias: `no-recursion`] — Do not recursively add directories to the archives. This is the inverse option of --recursive

  Default value: `false`
* `--overwrite` — Overwrite file

  Default value: `false`
* `--no-overwrite` — Do not overwrite files. This is the inverse option of --overwrite

  Default value: `false`
* `--keep-dir` — Include directories in archive (default)

  Default value: `false`
* `--no-keep-dir` — Do not archive directories. This is the inverse option of --keep-dir

  Default value: `false`
* `--keep-timestamp` [alias: `preserve-timestamps`] — Preserve file timestamps

  Default value: `false`
* `--no-keep-timestamp` [alias: `no-preserve-timestamps`] — Do not archive timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Preserve file permissions

  Default value: `false`
* `--no-keep-permission` [alias: `no-preserve-permissions`] — Do not archive permissions of files. This is the inverse option of --preserve-permissions

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Preserve extended attributes

  Default value: `false`
* `--no-keep-xattr` [alias: `no-preserve-xattrs`] — Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Preserve ACLs

  Default value: `false`
* `--no-keep-acl` [alias: `no-preserve-acls`] — Do not archive ACLs. This is the inverse option of --keep-acl

  Default value: `false`
* `--split <size>` — Splits archive by given size in bytes (minimum 64B)
* `--solid` — Compress multiple files together for better compression ratio

  Default value: `false`
* `--uname <NAME>` — Set user name for archive entries
* `--gname <NAME>` — Set group name for archive entries
* `--uid <ID>` — Overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id
* `--gid <ID>` — Overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id
* `--strip-components <N>` — Remove the specified number of leading path elements when storing paths
* `--numeric-owner` — This is equivalent to --uname "" --gname "". It causes user and group names to not be stored in the archive

  Default value: `false`
* `--ctime <DATETIME>` — Overrides the creation time read from disk
* `--clamp-ctime` — Clamp the creation time of the entries to the specified time by --ctime

  Default value: `false`
* `--atime <DATETIME>` — Overrides the access time read from disk
* `--clamp-atime` — Clamp the access time of the entries to the specified time by --atime

  Default value: `false`
* `--mtime <DATETIME>` — Overrides the modification time read from disk
* `--clamp-mtime` — Clamp the modification time of the entries to the specified time by --mtime

  Default value: `false`
* `--older-ctime <DATETIME>` — Only include files and directories older than the specified date. This compares ctime entries.
* `--older-mtime <DATETIME>` — Only include files and directories older than the specified date. This compares mtime entries.
* `--newer-ctime <DATETIME>` — Only include files and directories newer than the specified date. This compares ctime entries.
* `--newer-mtime <DATETIME>` — Only include files and directories newer than the specified date. This compares mtime entries.
* `--newer-ctime-than <FILE>` — Only include files and directories newer than the specified file. This compares ctime entries.
* `--newer-mtime-than <FILE>` — Only include files and directories newer than the specified file. This compares mtime entries.
* `--older-ctime-than <FILE>` — Only include files and directories older than the specified file. This compares ctime entries.
* `--older-mtime-than <FILE>` — Only include files and directories older than the specified file. This compares mtime entries.
* `--files-from <FILE>` — Read archiving files from given path
* `--files-from-stdin` — Read archiving files from stdin

  Default value: `false`
* `--include <PATTERN>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions
* `--exclude <PATTERN>` — Exclude path glob
* `--exclude-from <FILE>` — Read exclude files from given path
* `--exclude-vcs` — Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`)

  Default value: `false`
* `--gitignore` — Ignore files from .gitignore

  Default value: `false`
* `--follow-links` [alias: `dereference`] — Follow symbolic links

  Default value: `false`
* `-H`, `--follow-command-links` — Follow symbolic links named on the command line

  Default value: `false`
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option
* `--store` — No compression

  Default value: `false`
* `--deflate <level>` — Use deflate for compression [possible level: 1-9, min, max]
* `--zstd <level>` — Use zstd for compression [possible level: 1-21, min, max]
* `--xz <level>` — Use xz for compression [possible level: 0-9, min, max]
* `--aes <cipher mode>` — Use aes for encryption

  Possible values: `cbc`, `ctr`

* `--camellia <cipher mode>` — Use camellia for encryption

  Possible values: `cbc`, `ctr`

* `--argon2 <PARAMS>` — Use argon2 for password hashing
* `--pbkdf2 <PARAMS>` — Use pbkdf2 for password hashing
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `-f`, `--file <ARCHIVE>` — Archive file path
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna append`

Append files to archive

**Usage:** `pna append [OPTIONS] --file <ARCHIVE> [FILES]...`

**Command Alias:** `a`

###### **Arguments:**

* `<FILES>` — Files or directories to process

###### **Options:**

* `--one-file-system` — Stay in the same file system when collecting files

  Default value: `false`
* `--nodump` — Exclude files with the nodump flag

  Default value: `false`
* `-r`, `--recursive` [alias: `recursion`] — Add the directory to the archive recursively

  Default value: `true`
* `--no-recursive` [alias: `no-recursion`] — Do not recursively add directories to the archives. This is the inverse option of --recursive

  Default value: `false`
* `--keep-dir` — Include directories in archive (default)

  Default value: `false`
* `--no-keep-dir` — Do not archive directories. This is the inverse option of --keep-dir

  Default value: `false`
* `--keep-timestamp` [alias: `preserve-timestamps`] — Preserve file timestamps

  Default value: `false`
* `--no-keep-timestamp` [alias: `no-preserve-timestamps`] — Do not archive timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Preserve file permissions

  Default value: `false`
* `--no-keep-permission` [alias: `no-preserve-permissions`] — Do not archive permissions of files. This is the inverse option of --preserve-permissions

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Preserve extended attributes

  Default value: `false`
* `--no-keep-xattr` [alias: `no-preserve-xattrs`] — Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Preserve ACLs

  Default value: `false`
* `--no-keep-acl` [alias: `no-preserve-acls`] — Do not archive ACLs. This is the inverse option of --keep-acl

  Default value: `false`
* `--uname <NAME>` — Set user name for archive entries
* `--gname <NAME>` — Set group name for archive entries
* `--uid <ID>` — Overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id
* `--gid <ID>` — Overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id
* `--strip-components <N>` — Remove the specified number of leading path elements when storing paths
* `--numeric-owner` — This is equivalent to --uname "" --gname "". It causes user and group names to not be stored in the archive

  Default value: `false`
* `--ctime <DATETIME>` — Overrides the creation time read from disk
* `--clamp-ctime` — Clamp the creation time of the entries to the specified time by --ctime

  Default value: `false`
* `--atime <DATETIME>` — Overrides the access time read from disk
* `--clamp-atime` — Clamp the access time of the entries to the specified time by --atime

  Default value: `false`
* `--mtime <DATETIME>` — Overrides the modification time read from disk
* `--clamp-mtime` — Clamp the modification time of the entries to the specified time by --mtime

  Default value: `false`
* `--older-ctime <DATETIME>` — Only include files and directories older than the specified date. This compares ctime entries.
* `--older-mtime <DATETIME>` — Only include files and directories older than the specified date. This compares mtime entries.
* `--newer-ctime <DATETIME>` — Only include files and directories newer than the specified date. This compares ctime entries.
* `--newer-mtime <DATETIME>` — Only include files and directories newer than the specified date. This compares mtime entries.
* `--newer-ctime-than <FILE>` — Only include files and directories newer than the specified file. This compares ctime entries.
* `--newer-mtime-than <FILE>` — Only include files and directories newer than the specified file. This compares mtime entries.
* `--older-ctime-than <FILE>` — Only include files and directories older than the specified file. This compares ctime entries.
* `--older-mtime-than <FILE>` — Only include files and directories older than the specified file. This compares mtime entries.
* `--files-from <FILE>` — Read archiving files from given path
* `--files-from-stdin` — Read archiving files from stdin

  Default value: `false`
* `--include <PATTERN>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions
* `--exclude <PATTERN>` — Exclude path glob
* `--exclude-from <FILE>` — Read exclude files from given path
* `--exclude-vcs` — Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`)

  Default value: `false`
* `--gitignore` — Ignore files from .gitignore

  Default value: `false`
* `--follow-links` [alias: `dereference`] — Follow symbolic links

  Default value: `false`
* `-H`, `--follow-command-links` — Follow symbolic links named on the command line

  Default value: `false`
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option
* `--store` — No compression

  Default value: `false`
* `--deflate <level>` — Use deflate for compression [possible level: 1-9, min, max]
* `--zstd <level>` — Use zstd for compression [possible level: 1-21, min, max]
* `--xz <level>` — Use xz for compression [possible level: 0-9, min, max]
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--aes <cipher mode>` — Use aes for encryption

  Possible values: `cbc`, `ctr`

* `--camellia <cipher mode>` — Use camellia for encryption

  Possible values: `cbc`, `ctr`

* `--argon2 <PARAMS>` — Use argon2 for password hashing
* `--pbkdf2 <PARAMS>` — Use pbkdf2 for password hashing
* `-f`, `--file <ARCHIVE>` — Archive file path
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna extract`

Extract files from archive

**Usage:** `pna extract [OPTIONS] --file <ARCHIVE> [FILES]...`

**Command Alias:** `x`

###### **Arguments:**

* `<FILES>` — Files or directories to process

###### **Options:**

* `--overwrite` — Overwrite file

  Default value: `false`
* `--no-overwrite` — Do not overwrite files. This is the inverse option of --overwrite

  Default value: `false`
* `--keep-newer-files` — Skip extracting files if a newer version already exists

  Default value: `false`
* `--keep-old-files` — Skip extracting files if they already exist

  Default value: `false`
* `--out-dir <DIRECTORY>` — Output directory of extracted files
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--keep-timestamp` [alias: `preserve-timestamps`] — Restore the timestamp of the files

  Default value: `false`
* `--no-keep-timestamp` [alias: `no-preserve-timestamps`] — Do not restore timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--mtime <DATETIME>` — Overrides the modification time
* `--clamp-mtime` — Clamp the modification time of the entries to the specified time by --mtime

  Default value: `false`
* `--ctime <DATETIME>` — Overrides the creation time
* `--clamp-ctime` — Clamp the creation time of the entries to the specified time by --ctime

  Default value: `false`
* `--atime <DATETIME>` — Overrides the access time
* `--clamp-atime` — Clamp the access time of the entries to the specified time by --atime

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Restore the permissions of the files

  Default value: `false`
* `--no-keep-permission` [alias: `no-preserve-permissions`] — Do not restore permissions of files. This is the inverse option of --preserve-permissions

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Restore the extended attributes of the files

  Default value: `false`
* `--no-keep-xattr` [alias: `no-preserve-xattrs`] — Do not restore extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Restore ACLs

  Default value: `false`
* `--no-keep-acl` [alias: `no-preserve-acls`] — Do not restore ACLs. This is the inverse option of --keep-acl

  Default value: `false`
* `--uname <NAME>` — Restore user from given name
* `--gname <NAME>` — Restore group from given name
* `--uid <ID>` — Overrides the user id in the archive; the user name in the archive will be ignored
* `--gid <ID>` — Overrides the group id in the archive; the group name in the archive will be ignored
* `--numeric-owner` — This is equivalent to --uname "" --gname "". It causes user and group names in the archive to be ignored in favor of the numeric user and group ids.

  Default value: `false`
* `--older-ctime <DATETIME>` — Only include files and directories older than the specified date. This compares ctime entries.
* `--older-mtime <DATETIME>` — Only include files and directories older than the specified date. This compares mtime entries.
* `--newer-ctime <DATETIME>` — Only include files and directories newer than the specified date. This compares ctime entries.
* `--newer-mtime <DATETIME>` — Only include files and directories newer than the specified date. This compares mtime entries.
* `--newer-ctime-than <file>` [alias: `newer-than`] — Only include files and directories newer than the specified file. This compares ctime entries.
* `--newer-mtime-than <file>` — Only include files and directories newer than the specified file. This compares mtime entries.
* `--older-ctime-than <file>` [alias: `older-than`] — Only include files and directories older than the specified file. This compares ctime entries.
* `--older-mtime-than <file>` — Only include files and directories older than the specified file. This compares mtime entries.
* `--missing-time <MISSING_TIME>` — Behavior for entries missing a timestamp needed by the time filters (unstable). Values: include, exclude, now, epoch, or a datetime. [default: include]
* `--include <PATTERN>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions
* `--exclude <PATTERN>` — Exclude path glob
* `--exclude-from <FILE>` — Read exclude files from given path
* `--exclude-vcs` — Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`)

  Default value: `false`
* `--files-from <FILE>` — Read extraction patterns from given path
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `--strip-components <N>` — Remove the specified number of leading path elements. Path names with fewer elements will be silently skipped
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option
* `--same-owner` — Try extracting files with the same ownership as exists in the archive

  Default value: `false`
* `--no-same-owner` — Extract files as yourself

  Default value: `false`
* `--allow-unsafe-links` — Allow extracting symbolic links and hard links that contain root or parent paths

  Default value: `false`
* `--no-allow-unsafe-links` — Do not allow extracting symbolic links and hard links that contain root or parent paths (default)

  Default value: `false`
* `--safe-writes` — Extract files atomically via temp file and rename

  Default value: `false`
* `--no-safe-writes` — Disable atomic extraction. This is the inverse option of --safe-writes

  Default value: `false`
* `-f`, `--file <ARCHIVE>` — Archive file path
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna list`

List files in archive

**Usage:** `pna list [OPTIONS] --file <ARCHIVE> [FILES]...`

**Command Aliases:** `l`, `ls`

###### **Arguments:**

* `<FILES>` — Files or directories to process

###### **Options:**

* `-l`, `--long` — Display extended file metadata as a table

  Default value: `false`
* `-h`, `--header` — Add a header row to each column

  Default value: `false`
* `--solid` — Show entries that are compressed together

  Default value: `false`
* `-@` — Display extended file attributes in a table

  Default value: `false`
* `-e` — Display ACLs in a table

  Default value: `false`
* `-O`, `--show-fflags` — Display file flags (uchg, nodump, hidden, etc.)

  Default value: `false`
* `--private` — Display private chunks in a table

  Default value: `false`
* `--numeric-owner` — Display user id and group id instead of user name and group name

  Default value: `false`
* `-T` — When used with the -l option, display complete time information for the entry, including month, day, hour, minute, second, and year

  Default value: `false`
* `--format <FORMAT>` — Display format [unstable: jsonl, bsdtar, csv, tsv]

  Possible values: `line`, `table`, `jsonl`, `tree`, `bsdtar`, `csv`, `tsv`

* `--time <TIME>` — Which timestamp field to list (modified, accessed, created)

  Possible values: `created`, `modified`, `accessed`

* `--older-ctime <OLDER_CTIME>` — Only include files and directories older than the specified date. This compares ctime entries.
* `--older-mtime <OLDER_MTIME>` — Only include files and directories older than the specified date. This compares mtime entries.
* `--newer-ctime <NEWER_CTIME>` — Only include files and directories newer than the specified date. This compares ctime entries.
* `--newer-mtime <NEWER_MTIME>` — Only include files and directories newer than the specified date. This compares mtime entries.
* `--newer-ctime-than <file>` [alias: `newer-than`] — Only include files and directories newer than the specified file. This compares ctime entries.
* `--newer-mtime-than <file>` — Only include files and directories newer than the specified file. This compares mtime entries.
* `--older-ctime-than <file>` [alias: `older-than`] — Only include files and directories older than the specified file. This compares ctime entries.
* `--older-mtime-than <file>` — Only include files and directories older than the specified file. This compares mtime entries.
* `--missing-time <MISSING_TIME>` — Behavior for entries missing a timestamp needed by the time filters (unstable). Values: include, exclude, now, epoch, or a datetime. [default: include]
* `-q` — Force printing of non-graphic characters in file names as the character '?'

  Default value: `false`
* `--classify` — Append file type indicators (/ for directories, @ for symlinks)

  Default value: `false`
* `--recursive` [alias: `recursion`] — Operate recursively on the content of directories (default)

  Default value: `true`
* `-n`, `--no-recursive` [aliases: `norecurse`, `no-recursion`] — Do not operate recursively on the content of directories

  Default value: `false`
* `--include <PATTERN>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions
* `--exclude <PATTERN>` — Exclude path glob
* `--exclude-from <FILE>` — Read exclude files from given path
* `--exclude-vcs` — Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`)

  Default value: `false`
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `-f`, `--file <ARCHIVE>` — Archive file path
* `--help` — Print help
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`



## `pna delete`

Delete entry from archive

**Usage:** `pna delete [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>` — Files or directories to process

###### **Options:**

* `--output <OUTPUT>` — Output file path
* `--files-from <FILE>` — Read deleting files from given path
* `--files-from-stdin` — Read deleting files from stdin

  Default value: `false`
* `--include <PATTERN>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions
* `--exclude <PATTERN>` — Exclude path glob
* `--exclude-from <FILE>` — Read exclude files from given path
* `--exclude-vcs` — Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`)

  Default value: `false`
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--unsolid` — Convert solid entries to regular entries

  Default value: `false`
* `--keep-solid` — Preserve solid entries without conversion

  Default value: `false`
* `-f`, `--file <ARCHIVE>` — Archive file path
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna split`

Split archive

**Usage:** `pna split [OPTIONS] --file <FILE>`

###### **Options:**

* `-f`, `--file <FILE>` — Archive file path
* `--out-dir <DIRECTORY>` — Output directory for split archives
* `--overwrite` — Overwrite file

  Default value: `false`
* `--no-overwrite` — Do not overwrite files. This is the inverse option of --overwrite

  Default value: `false`
* `--max-size <size>` — Maximum size in bytes of split archive (minimum 64B)
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna concat`

Concat archives

**Usage:** `pna concat [OPTIONS] --files <FILES>`

###### **Options:**

* `--overwrite` — Overwrite file

  Default value: `false`
* `--no-overwrite` — Do not overwrite files. This is the inverse option of --overwrite

  Default value: `false`
* `-f`, `--files <FILES>` — Archive files to concatenate
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna strip`

Strip entries metadata

**Usage:** `pna strip [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>` — Files or directories to process

###### **Options:**

* `--keep-timestamp` [alias: `preserve-timestamps`] — Preserve file timestamps

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Preserve file permissions

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Preserve extended attributes

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Preserve ACLs

  Default value: `false`
* `--keep-private <CHUNK_TYPE>` [alias: `preserve-private_chunks`] — Keep private chunks. If no CHUNK_TYPE is specified, all private chunks are kept
* `--unsolid` — Convert solid entries to regular entries

  Default value: `false`
* `--keep-solid` — Preserve solid entries without conversion

  Default value: `false`
* `--output <OUTPUT>` — Output file path
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `-f`, `--file <ARCHIVE>` — Archive file path
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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

* `-f`, `--file <ARCHIVE>` — Archive file path
* `--output <OUTPUT>` — Output archive file path
* `--by <KEY>` — Sort key in format KEY[:ORDER] (e.g., name, mtime:desc) [keys: name, ctime, mtime, atime] [orders: asc, desc]

  Default value: `name`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna migrate`

Upgrade archives created by older PNA versions

**Usage:** `pna migrate [OPTIONS] --file <ARCHIVE> --output <OUTPUT>`

###### **Options:**

* `--unsolid` — Convert solid entries to regular entries

  Default value: `false`
* `--keep-solid` — Preserve solid entries without conversion

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `-f`, `--file <ARCHIVE>`
* `--output <OUTPUT>` — Output file path
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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

* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna xattr get`

Get extended attributes of entries

**Usage:** `pna xattr get [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>` — Files or directories to process

###### **Options:**

* `-f`, `--file <ARCHIVE>` — Archive file path
* `-n`, `--name <NAME>` — Dump the value of the named extended attribute
* `-d`, `--dump` — Dump the values of all matched extended attributes

  Default value: `false`
* `-m`, `--match <pattern>` — Only include attributes with names matching the regular expression pattern. Specify '-' for including all attributes
* `-e`, `--encoding <ENCODING>` — Encode values after retrieving them

  Possible values: `text`, `hex`, `base64`

* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna xattr set`

Set extended attributes of entries

**Usage:** `pna xattr set [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>` — Files or directories to process

###### **Options:**

* `-f`, `--file <ARCHIVE>` — Archive file path
* `-n`, `--name <NAME>` — Name of extended attribute
* `-v`, `--value <VALUE>` — Value of extended attribute
* `-x`, `--remove <NAME>` — Remove extended attribute
* `--restore <FILE>` — Restores extended attributes from file. The file must be in the format generated by the pna xattr get command with the --dump option. If a dash (-) is given as the file name, reads from standard input
* `--unsolid` — Convert solid entries to regular entries

  Default value: `false`
* `--keep-solid` — Preserve solid entries without conversion

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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

* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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

* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna compat`

Compatibility interface for other archive tools

**Usage:** `pna compat [OPTIONS]
       pna compat <COMMAND>`

###### **Subcommands:**

* `bsdtar` — bsdtar-like CLI semantics for PNA archives
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna compat bsdtar`

bsdtar-like CLI semantics for PNA archives

**Usage:** `pna compat bsdtar [OPTIONS] <--create|--extract|--list|--append|--update> [FILES]...`

###### **Arguments:**

* `<FILES>` — Files or patterns

###### **Options:**

* `--one-file-system` — Stay in the same file system when collecting files

  Default value: `false`
* `--nodump` — Exclude files with the nodump flag

  Default value: `false`
* `-c`, `--create` — Create archive

  Default value: `false`
* `-x`, `--extract` — Extract archive

  Default value: `false`
* `-t`, `--list` — List files in archive

  Default value: `false`
* `-q`, `--fast-read` — Performance optimization for list/extract: stop after the first match for each operand and ignore later duplicates

  Default value: `false`
* `-r`, `--append` — Append files to archive

  Default value: `false`
* `-u`, `--update` — Update archive with newer files

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
* `--keep-dir` — Include directories in archive (default)

  Default value: `false`
* `--no-keep-dir` — Do not archive directories. This is the inverse option of --keep-dir

  Default value: `false`
* `--keep-timestamp` [alias: `preserve-timestamps`] — Preserve file timestamps

  Default value: `false`
* `-m`, `--no-keep-timestamp` [aliases: `no-preserve-timestamps`, `modification_time`] — Do not archive timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--no-same-permissions` [aliases: `no-preserve-permissions`, `no-permissions`] — Do not store file permissions (mode bits) in the archive

  Default value: `false`
* `-p`, `--same-permissions` [alias: `preserve-permissions`] — Restore file permissions (mode, ACLs, xattrs, fflags, mac-metadata, but NOT ownership) (extract only)

  Default value: `false`
* `--keep-xattr` [aliases: `preserve-xattrs`, `xattrs`] — Preserve extended attributes

  Default value: `false`
* `--no-keep-xattr` [aliases: `no-preserve-xattrs`, `no-xattrs`] — Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [aliases: `preserve-acls`, `acls`] — Preserve ACLs

  Default value: `false`
* `--no-keep-acl` [aliases: `no-preserve-acls`, `no-acls`] — Do not archive ACLs. This is the inverse option of --keep-acl

  Default value: `false`
* `--keep-fflags` [aliases: `preserve-fflags`, `fflags`] — Archiving the file flags of the files

  Default value: `false`
* `--no-keep-fflags` [aliases: `no-preserve-fflags`, `no-fflags`] — Do not archive file flags of files. This is the inverse option of --keep-fflags

  Default value: `false`
* `--mac-metadata` — Archive and extract Mac metadata (extended attributes and ACLs)

  Default value: `false`
* `--no-mac-metadata` — Do not archive or extract Mac metadata. This is the inverse option of --mac-metadata

  Default value: `false`
* `--solid` — Compress multiple files together for better compression ratio

  Default value: `false`
* `--store` — No compression

  Default value: `false`
* `--deflate` [alias: `zlib`] — Use deflate for compression

  Default value: `false`
* `--zstd` — Use zstd for compression

  Default value: `false`
* `-J`, `--xz` — Use xz for compression

  Default value: `false`
* `--aes <cipher mode>` — Use aes for encryption

  Possible values: `cbc`, `ctr`

* `--camellia <cipher mode>` — Use camellia for encryption

  Possible values: `cbc`, `ctr`

* `--argon2 <PARAMS>` — Use argon2 for password hashing
* `--pbkdf2 <PARAMS>` — Use pbkdf2 for password hashing
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--options <OPTIONS>` — Comma-separated list of options. Format: key=value or module:key=value. Supported: compression-level. Modules: deflate, zstd, xz
* `--include <PATTERN>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions
* `--exclude <PATTERN>` — Exclude path glob
* `-X`, `--exclude-from <FILE>` — Read exclude files from given path
* `--exclude-vcs` — Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`)

  Default value: `false`
* `--gitignore` — Ignore files from .gitignore

  Default value: `false`
* `-L`, `--follow-links` [alias: `dereference`] — Follow symbolic links

  Default value: `false`
* `-H`, `--follow-command-links` — Follow symbolic links named on the command line

  Default value: `false`
* `-l`, `--check-links` [alias: `check-links`] — Warn if not all links to each file are archived (create mode)

  Default value: `false`
* `--out-dir <DIRECTORY>` — Output directory of extracted files
* `--strip-components <N>` — Remove the specified number of leading path elements. Path names with fewer elements will be silently skipped
* `--owner <NAME[:ID]>` — Use the provided owner, if uid is not provided, name can be either a user name or numeric id. See the --uname option for details.
* `--uname <NAME>` — On create, archiving user to the entries from given name. On extract, restore user from given name
* `--gname <NAME>` — On create, archiving group to the entries from given name. On extract, restore group from given name
* `--uid <ID>` — On create, this overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id. On extract, this overrides the user id in the archive; the user name in the archive will be ignored
* `--gid <ID>` — On create, this overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id. On extract, this overrides the group id in the archive; the group name in the archive will be ignored
* `--group <NAME[:ID]>` — Use the provided group, if gid is not provided, name can be either a group name or numeric id. See the --gname option for details.
* `--numeric-owner` — This is equivalent to --uname "" --gname "". On create, it causes user and group names to not be stored in the archive. On extract, it causes user and group names in the archive to be ignored in favor of the numeric user and group ids.

  Default value: `false`
* `--ctime <DATETIME>` — Overrides the creation time
* `--clamp-ctime` — Clamp the creation time of the entries to the specified time by --ctime

  Default value: `false`
* `--atime <DATETIME>` — Overrides the access time
* `--clamp-atime` — Clamp the access time of the entries to the specified time by --atime

  Default value: `false`
* `--mtime <DATETIME>` — Overrides the modification time
* `--clamp-mtime` — Clamp the modification time of the entries to the specified time by --mtime

  Default value: `false`
* `--older-ctime <DATETIME>` — Only include files and directories older than the specified date. This compares ctime entries.
* `--older-mtime <DATETIME>` [alias: `older`] — Only include files and directories older than the specified date. This compares mtime entries.
* `--newer-ctime <DATETIME>` — Only include files and directories newer than the specified date. This compares ctime entries.
* `--newer-mtime <DATETIME>` [alias: `newer`] — Only include files and directories newer than the specified date. This compares mtime entries.
* `--newer-ctime-than <FILE>` — Only include files and directories newer than the specified file. This compares ctime entries.
* `--newer-mtime-than <FILE>` [alias: `newer-than`] — Only include files and directories newer than the specified file. This compares mtime entries.
* `--older-ctime-than <FILE>` — Only include files and directories older than the specified file. This compares ctime entries.
* `--older-mtime-than <FILE>` [alias: `older-than`] — Only include files and directories older than the specified file. This compares mtime entries.
* `-T`, `--files-from <FILE>` — Read archiving files from given path
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option
* `--same-owner` — Try extracting files with the same ownership as exists in the archive

  Default value: `false`
* `--no-same-owner` — Extract files as yourself

  Default value: `false`
* `-C`, `--cd <DIRECTORY>` [alias: `directory`] — Change directory before adding the following files
* `-O`, `--to-stdout` — Write extracted file data to standard output instead of the file system

  Default value: `false`
* `--allow-unsafe-links` — Allow extracting symbolic links and hard links that contain root or parent paths (default)

  Default value: `false`
* `--no-allow-unsafe-links` — Do not allow extracting symbolic links and hard links that contain root or parent paths

  Default value: `false`
* `--chroot` — chroot() to the current directory after processing any --cd options and before extracting any files (requires root privileges)

  Default value: `false`
* `-P`, `--absolute-paths` — Do not strip leading '/' or '..' from member names and link targets

  Default value: `false`
* `-f`, `--file <FILE>` — Read the archive from or write the archive to the specified file. The filename can be - for standard input or standard output.
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `-v` — Verbose

  Default value: `false`
* `--ignore-zeros` — Continue reading concatenated archives after the first archive ends instead of stopping at the first archive boundary

  Default value: `false`
* `--version` — Print version
* `-h`, `--help` — Print help
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`



## `pna compat help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna compat help [COMMAND]`

###### **Subcommands:**

* `bsdtar` — bsdtar-like CLI semantics for PNA archives
* `help` — Print this message or the help of the given subcommand(s)



## `pna compat help bsdtar`

bsdtar-like CLI semantics for PNA archives

**Usage:** `pna compat help bsdtar`



## `pna compat help help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna compat help help`



## `pna experimental`

Unstable experimental commands; behavior and interface may change or be removed

**Usage:** `pna experimental [OPTIONS]
       pna experimental <COMMAND>`

###### **Subcommands:**

* `update` — Update entries in archive
* `chown` — Change owner
* `chmod` — Change mode
* `acl` — Manipulate ACLs of entries
* `migrate` — Upgrade archives created by older PNA versions (stabilized, use `pna migrate` command instead. this command will be removed in the future)
* `chunk` — Chunk level operation
* `diff` — Compare archive entries with filesystem
* `verify` — Verify archive integrity
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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

* `<FILES>` — Files or directories to process

###### **Options:**

* `--output <OUTPUT>` — Output file path
* `--one-file-system` — Stay in the same file system when collecting files

  Default value: `false`
* `--nodump` — Exclude files with the nodump flag

  Default value: `false`
* `-r`, `--recursive` [alias: `recursion`] — Add the directory to the archive recursively

  Default value: `true`
* `--no-recursive` [alias: `no-recursion`] — Do not recursively add directories to the archives. This is the inverse option of --recursive

  Default value: `false`
* `--keep-dir` — Include directories in archive (default)

  Default value: `false`
* `--no-keep-dir` — Do not archive directories. This is the inverse option of --keep-dir

  Default value: `false`
* `--keep-timestamp` [alias: `preserve-timestamps`] — Preserve file timestamps

  Default value: `false`
* `--no-keep-timestamp` [alias: `no-preserve-timestamps`] — Do not archive timestamp of files. This is the inverse option of --preserve-timestamps

  Default value: `false`
* `--keep-permission` [alias: `preserve-permissions`] — Preserve file permissions

  Default value: `false`
* `--no-keep-permission` [alias: `no-preserve-permissions`] — Do not archive permissions of files. This is the inverse option of --preserve-permissions

  Default value: `false`
* `--keep-xattr` [alias: `preserve-xattrs`] — Preserve extended attributes

  Default value: `false`
* `--no-keep-xattr` [alias: `no-preserve-xattrs`] — Do not archive extended attributes of files. This is the inverse option of --preserve-xattrs

  Default value: `false`
* `--keep-acl` [alias: `preserve-acls`] — Preserve ACLs

  Default value: `false`
* `--no-keep-acl` [alias: `no-preserve-acls`] — Do not archive ACLs. This is the inverse option of --keep-acl

  Default value: `false`
* `--uname <NAME>` — Set user name for archive entries
* `--gname <NAME>` — Set group name for archive entries
* `--uid <ID>` — Overrides the user id read from disk; if --uname is not also specified, the user name will be set to match the user id
* `--gid <ID>` — Overrides the group id read from disk; if --gname is not also specified, the group name will be set to match the group id
* `--strip-components <N>` — Remove the specified number of leading path elements when storing paths
* `--numeric-owner` — This is equivalent to --uname "" --gname "". It causes user and group names to not be stored in the archive

  Default value: `false`
* `--ctime <DATETIME>` — Overrides the creation time read from disk
* `--clamp-ctime` — Clamp the creation time of the entries to the specified time by --ctime

  Default value: `false`
* `--atime <DATETIME>` — Overrides the access time read from disk
* `--clamp-atime` — Clamp the access time of the entries to the specified time by --atime

  Default value: `false`
* `--mtime <DATETIME>` — Overrides the modification time read from disk
* `--clamp-mtime` — Clamp the modification time of the entries to the specified time by --mtime

  Default value: `false`
* `--older-ctime <DATETIME>` — Only include files and directories older than the specified date. This compares ctime entries.
* `--older-mtime <DATETIME>` — Only include files and directories older than the specified date. This compares mtime entries.
* `--newer-ctime <DATETIME>` — Only include files and directories newer than the specified date. This compares ctime entries.
* `--newer-mtime <DATETIME>` — Only include files and directories newer than the specified date. This compares mtime entries.
* `--newer-ctime-than <FILE>` — Only include files and directories newer than the specified file. This compares ctime entries.
* `--newer-mtime-than <FILE>` — Only include files and directories newer than the specified file. This compares mtime entries.
* `--older-ctime-than <FILE>` — Only include files and directories older than the specified file. This compares ctime entries.
* `--older-mtime-than <FILE>` — Only include files and directories older than the specified file. This compares mtime entries.
* `--missing-time <MISSING_TIME>` — Behavior when a timestamp needed for time filtering or update staleness judgment is missing (unstable). Values: include, exclude, now, epoch, or a datetime. [default: include]
* `--files-from <FILE>` — Read archiving files from given path
* `--files-from-stdin` — Read archiving files from stdin

  Default value: `false`
* `--include <PATTERN>` — Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions
* `--exclude <PATTERN>` — Exclude path glob
* `--exclude-from <FILE>` — Read exclude files from given path
* `--exclude-vcs` — Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`)

  Default value: `false`
* `-s <PATTERN>` — Modify file or archive member names according to pattern that like BSD tar -s option
* `--transform <PATTERN>` [alias: `xform`] — Modify file or archive member names according to pattern that like GNU tar -transform option
* `--store` — No compression

  Default value: `false`
* `--deflate <level>` — Use deflate for compression [possible level: 1-9, min, max]
* `--zstd <level>` — Use zstd for compression [possible level: 1-21, min, max]
* `--xz <level>` — Use xz for compression [possible level: 0-9, min, max]
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--aes <cipher mode>` — Use aes for encryption

  Possible values: `cbc`, `ctr`

* `--camellia <cipher mode>` — Use camellia for encryption

  Possible values: `cbc`, `ctr`

* `--argon2 <PARAMS>` — Use argon2 for password hashing
* `--pbkdf2 <PARAMS>` — Use pbkdf2 for password hashing
* `--unsolid` — Convert solid entries to regular entries

  Default value: `false`
* `--keep-solid` — Preserve solid entries without conversion

  Default value: `false`
* `-f`, `--file <ARCHIVE>` — Archive file path
* `--null` — Filenames or patterns are separated by null characters, not by newlines

  Default value: `false`
* `--gitignore` — Ignore files from .gitignore

  Default value: `false`
* `--follow-links` [alias: `dereference`] — Follow symbolic links

  Default value: `false`
* `-H`, `--follow-command-links` — Follow symbolic links named on the command line

  Default value: `false`
* `--sync` — Synchronize archive with source: replace updated entries instead of appending, and remove entries for files that no longer exist on disk.

  Default value: `false`
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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
* `--unsolid` — Convert solid entries to regular entries

  Default value: `false`
* `--keep-solid` — Preserve solid entries without conversion

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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
* `--unsolid` — Convert solid entries to regular entries

  Default value: `false`
* `--keep-solid` — Preserve solid entries without conversion

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental acl`

Manipulate ACLs of entries

**Usage:** `pna experimental acl [OPTIONS]
       pna experimental acl <COMMAND>`

###### **Subcommands:**

* `get` — Get acl of entries
* `set` — Set acl of entries
* `help` — Print this message or the help of the given subcommand(s)

###### **Options:**

* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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

* `<FILES>` — Files or directories to process

###### **Options:**

* `--platform <PLATFORM>` — Display specified ACL platform
* `-n`, `--numeric` — List numeric user and group IDs

  Default value: `false`
* `-f`, `--file <ARCHIVE>` — Archive file path
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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

* `<FILES>` — Files or directories to process

###### **Options:**

* `-f`, `--file <ARCHIVE>` — Archive file path
* `--set <SET>` — Set the ACL on the specified file.
* `-m`, `--modify <MODIFY>` — Modify the ACL on the specified file. New entries will be added, and existing entries will be modified according to the entries argument.
* `-x`, `--remove <REMOVE>` — Remove the ACL entries specified there from the access or default ACL of the specified files.
* `--platform <PLATFORM>` — Target ACL platform

  Default value: ``
* `--restore <RESTORE>` — Restore a permission backup created by `pna acl get *` or similar. All permissions of a complete directory subtree are restored using this mechanism. If a dash (-) is given as the file name, reads from standard input
* `--unsolid` — Convert solid entries to regular entries

  Default value: `false`
* `--keep-solid` — Preserve solid entries without conversion

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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

Upgrade archives created by older PNA versions (stabilized, use `pna migrate` command instead. this command will be removed in the future)

**Usage:** `pna experimental migrate [OPTIONS] --file <ARCHIVE> --output <OUTPUT>`

###### **Options:**

* `--unsolid` — Convert solid entries to regular entries

  Default value: `false`
* `--keep-solid` — Preserve solid entries without conversion

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `-f`, `--file <ARCHIVE>`
* `--output <OUTPUT>` — Output file path
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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

* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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
* `--type <TYPE>` — Only list chunks of the specified type
* `--exclude-type <TYPE>` — Do not list chunks of the specified type
* `-f`, `--file <ARCHIVE>`
* `--help` — Print help
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

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



## `pna experimental diff`

Compare archive entries with filesystem

**Usage:** `pna experimental diff [OPTIONS] --file <ARCHIVE> [FILES]...`

###### **Arguments:**

* `<FILES>` — Files or directories to process

###### **Options:**

* `-f`, `--file <ARCHIVE>` — Archive file path
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--full-compare` — Compare directory mtime and ownership (by default, only mode is compared for directories)

  Default value: `false`
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help



## `pna experimental verify`

Verify archive integrity

**Usage:** `pna experimental verify [OPTIONS] --file <ARCHIVE>`

Note: for encrypted entries, a wrong password is indistinguishable from corruption.

###### **Options:**

* `-f`, `--file <ARCHIVE>` — Archive file path
* `--fast` — Verify chunk structure and CRC32 only, without decoding entry data. Solid blocks are still decoded because enumerating their entries requires decompression and decryption, so stream corruption inside a solid block is detected even with --fast. Encrypted normal entries are counted as ok because nothing is decoded, so the skipped category does not apply.

  Default value: `false`
* `--password <PASSWORD>` [alias: `passphrase`] — Password of archive. If password is not given it's asked from the tty
* `--password-file <FILE>` — Read password from the specified file (entire contents). Files containing newlines or non-UTF-8 content emit a warning; use --password-file-raw if the full file content is intentionally the password
* `--password-file-raw <FILE>` — Read password from the specified file as-is (entire file content, including newlines)
* `--quiet` — Make some output more quiet (alias for --log-level off)

  Default value: `false`
* `--verbose` — Make some output more verbose (alias for --log-level debug)

  Default value: `false`
* `--log-level <LEVEL>` — Set the log level

  Default value: `warn`

  Possible values: `off`, `error`, `warn`, `info`, `debug`, `trace`

* `--color <WHEN>` — Control color output

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--unstable` — Enable experimental options. Required for flags marked as unstable; behavior may change or be removed.

  Default value: `false`
* `-h`, `--help` — Print help (see a summary with '-h')



## `pna experimental help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna experimental help [COMMAND]`

###### **Subcommands:**

* `update` — Update entries in archive
* `chown` — Change owner
* `chmod` — Change mode
* `acl` — Manipulate ACLs of entries
* `migrate` — Upgrade archives created by older PNA versions (stabilized, use `pna migrate` command instead. this command will be removed in the future)
* `chunk` — Chunk level operation
* `diff` — Compare archive entries with filesystem
* `verify` — Verify archive integrity
* `help` — Print this message or the help of the given subcommand(s)



## `pna experimental help update`

Update entries in archive

**Usage:** `pna experimental help update`



## `pna experimental help chown`

Change owner

**Usage:** `pna experimental help chown`



## `pna experimental help chmod`

Change mode

**Usage:** `pna experimental help chmod`



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

Upgrade archives created by older PNA versions (stabilized, use `pna migrate` command instead. this command will be removed in the future)

**Usage:** `pna experimental help migrate`



## `pna experimental help chunk`

Chunk level operation

**Usage:** `pna experimental help chunk [COMMAND]`

###### **Subcommands:**

* `list` — List chunks



## `pna experimental help chunk list`

List chunks

**Usage:** `pna experimental help chunk list`



## `pna experimental help diff`

Compare archive entries with filesystem

**Usage:** `pna experimental help diff`



## `pna experimental help verify`

Verify archive integrity

**Usage:** `pna experimental help verify`



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
* `delete` — Delete entry from archive
* `split` — Split archive
* `concat` — Concat archives
* `strip` — Strip entries metadata
* `sort` — Sort entries in archive
* `migrate` — Upgrade archives created by older PNA versions
* `xattr` — Manipulate extended attributes
* `complete` — Generate shell auto complete
* `bug-report` — Generate bug report template
* `compat` — Compatibility interface for other archive tools
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



## `pna help delete`

Delete entry from archive

**Usage:** `pna help delete`



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



## `pna help migrate`

Upgrade archives created by older PNA versions

**Usage:** `pna help migrate`



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



## `pna help compat`

Compatibility interface for other archive tools

**Usage:** `pna help compat [COMMAND]`

###### **Subcommands:**

* `bsdtar` — bsdtar-like CLI semantics for PNA archives



## `pna help compat bsdtar`

bsdtar-like CLI semantics for PNA archives

**Usage:** `pna help compat bsdtar`



## `pna help experimental`

Unstable experimental commands; behavior and interface may change or be removed

**Usage:** `pna help experimental [COMMAND]`

###### **Subcommands:**

* `update` — Update entries in archive
* `chown` — Change owner
* `chmod` — Change mode
* `acl` — Manipulate ACLs of entries
* `migrate` — Upgrade archives created by older PNA versions (stabilized, use `pna migrate` command instead. this command will be removed in the future)
* `chunk` — Chunk level operation
* `diff` — Compare archive entries with filesystem
* `verify` — Verify archive integrity



## `pna help experimental update`

Update entries in archive

**Usage:** `pna help experimental update`



## `pna help experimental chown`

Change owner

**Usage:** `pna help experimental chown`



## `pna help experimental chmod`

Change mode

**Usage:** `pna help experimental chmod`



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

Upgrade archives created by older PNA versions (stabilized, use `pna migrate` command instead. this command will be removed in the future)

**Usage:** `pna help experimental migrate`



## `pna help experimental chunk`

Chunk level operation

**Usage:** `pna help experimental chunk [COMMAND]`

###### **Subcommands:**

* `list` — List chunks



## `pna help experimental chunk list`

List chunks

**Usage:** `pna help experimental chunk list`



## `pna help experimental diff`

Compare archive entries with filesystem

**Usage:** `pna help experimental diff`



## `pna help experimental verify`

Verify archive integrity

**Usage:** `pna help experimental verify`



## `pna help help`

Print this message or the help of the given subcommand(s)

**Usage:** `pna help help`



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
