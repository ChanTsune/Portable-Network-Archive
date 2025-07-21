#!/usr/bin/env bats

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  # Create test directory structure with VCS files
  assert_make_dir "in" 0755
  pushd "in" || exit 1

  assert_make_file "file" 0644 ""
  assert_make_dir "dir" 0755
  assert_make_dir "CVS" 0755
  assert_make_file "CVS/fileattr" 0644 ""
  assert_make_file ".cvsignore" 0644 ""
  assert_make_dir "RCS" 0755
  assert_make_file "RCS/somefile" 0655 ""
  assert_make_dir "SCCS" 0755
  assert_make_file "SCCS/somefile" 0655 ""
  assert_make_dir ".svn" 0755
  assert_make_file ".svn/format" 0655 ""
  assert_make_dir ".git" 0755
  assert_make_file ".git/config" 0655 ""
  assert_make_file ".gitignore" 0644 ""
  assert_make_file ".gitattributes" 0644 ""
  assert_make_file ".gitmodules" 0644 ""
  assert_make_dir ".arch-ids" 0755
  assert_make_file ".arch-ids/somefile" 0644 ""
  assert_make_dir "{arch}" 0755
  assert_make_file "{arch}/somefile" 0644 ""
  assert_make_file "=RELEASE-ID" 0644 ""
  assert_make_file "=meta-update" 0644 ""
  assert_make_file "=update" 0644 ""
  assert_make_dir ".bzr" 0755
  assert_make_dir ".bzr/checkout" 0755
  assert_make_file ".bzrignore" 0644 ""
  assert_make_file ".bzrtags" 0644 ""
  assert_make_dir ".hg" 0755
  assert_make_file ".hg/dirstate" 0644 ""
  assert_make_file ".hgignore" 0644 ""
  assert_make_file ".hgtags" 0644 ""
  assert_make_dir "_darcs" 0755
  assert_make_file "_darcs/format" 0644 ""

  popd || exit 1

  # Create archives with and without --exclude-vcs
  run bash -c "$EXECUTABLE -c -C in -f included.tar ."
  assert_success
  run bash -c "$EXECUTABLE -c --exclude-vcs -C in -f excluded.tar ."
  assert_success
}

teardown_file() {
  popd || exit 1
}

setup() {
  TEST_DIR="test$BATS_TEST_NUMBER"
  assert_make_dir "$TEST_DIR" 0755
  pushd "$TEST_DIR" || exit 1
}

teardown() {
  popd || exit 1
}

@test "Test 1: No flags, archive with vcs files" {
  run bash -c "$EXECUTABLE -x -C . -f ../included.tar"
  assert_success

  assert_file_exist "file"
  assert_dir_exist "dir"
  assert_dir_exist "CVS"
  assert_file_exist "CVS/fileattr"
  assert_file_exist ".cvsignore"
  assert_dir_exist "RCS"
  assert_file_exist "RCS/somefile"
  assert_dir_exist "SCCS"
  assert_file_exist "SCCS/somefile"
  assert_dir_exist ".svn"
  assert_file_exist ".svn/format"
  assert_dir_exist ".git"
  assert_file_exist ".git/config"
  assert_file_exist ".gitignore"
  assert_file_exist ".gitattributes"
  assert_file_exist ".gitmodules"
  assert_dir_exist ".arch-ids"
  assert_file_exist ".arch-ids/somefile"
  assert_dir_exist "{arch}"
  assert_file_exist "{arch}/somefile"
  assert_file_exist "=RELEASE-ID"
  assert_file_exist "=meta-update"
  assert_file_exist "=update"
  assert_dir_exist ".bzr"
  assert_dir_exist ".bzr/checkout"
  assert_file_exist ".bzrignore"
  assert_file_exist ".bzrtags"
  assert_dir_exist ".hg"
  assert_file_exist ".hg/dirstate"
  assert_file_exist ".hgignore"
  assert_file_exist ".hgtags"
  assert_dir_exist "_darcs"
  assert_file_exist "_darcs/format"
}

@test "Test 2: --exclude-vcs, archive with vcs files" {
  run bash -c "$EXECUTABLE -x --exclude-vcs -C . -f ../included.tar"
  assert_success

  assert_file_exist "file"
  assert_dir_exist "dir"
  assert_file_not_exist "CVS"
  assert_file_not_exist "CVS/fileattr"
  assert_file_not_exist ".cvsignore"
  assert_file_not_exist "RCS"
  assert_file_not_exist "RCS/somefile"
  assert_file_not_exist "SCCS"
  assert_file_not_exist "SCCS/somefile"
  assert_file_not_exist ".svn"
  assert_file_not_exist ".svn/format"
  assert_file_not_exist ".git"
  assert_file_not_exist ".git/config"
  assert_file_not_exist ".gitignore"
  assert_file_not_exist ".gitattributes"
  assert_file_not_exist ".gitmodules"
  assert_file_not_exist ".arch-ids"
  assert_file_not_exist ".arch-ids/somefile"
  assert_file_not_exist "{arch}"
  assert_file_not_exist "{arch}/somefile"
  assert_file_not_exist "=RELEASE-ID"
  assert_file_not_exist "=meta-update"
  assert_file_not_exist "=update"
  assert_file_not_exist ".bzr"
  assert_file_not_exist ".bzr/checkout"
  assert_file_not_exist ".bzrignore"
  assert_file_not_exist ".bzrtags"
  assert_file_not_exist ".hg"
  assert_file_not_exist ".hg/dirstate"
  assert_file_not_exist ".hgignore"
  assert_file_not_exist ".hgtags"
  assert_file_not_exist "_darcs"
  assert_file_not_exist "_darcs/format"
}

@test "Test 3: --exclude-vcs, archive without vcs files" {
  run bash -c "$EXECUTABLE -x --exclude-vcs -C . -f ../excluded.tar"
  assert_success

  assert_file_exist "file"
  assert_dir_exist "dir"
  assert_file_not_exist "CVS"
  assert_file_not_exist "CVS/fileattr"
  assert_file_not_exist ".cvsignore"
  assert_file_not_exist "RCS"
  assert_file_not_exist "RCS/somefile"
  assert_file_not_exist "SCCS"
  assert_file_not_exist "SCCS/somefile"
  assert_file_not_exist ".svn"
  assert_file_not_exist ".svn/format"
  assert_file_not_exist ".git"
  assert_file_not_exist ".git/config"
  assert_file_not_exist ".gitignore"
  assert_file_not_exist ".gitattributes"
  assert_file_not_exist ".gitmodules"
  assert_file_not_exist ".arch-ids"
  assert_file_not_exist ".arch-ids/somefile"
  assert_file_not_exist "{arch}"
  assert_file_not_exist "{arch}/somefile"
  assert_file_not_exist "=RELEASE-ID"
  assert_file_not_exist "=meta-update"
  assert_file_not_exist "=update"
  assert_file_not_exist ".bzr"
  assert_file_not_exist ".bzr/checkout"
  assert_file_not_exist ".bzrignore"
  assert_file_not_exist ".bzrtags"
  assert_file_not_exist ".hg"
  assert_file_not_exist ".hg/dirstate"
  assert_file_not_exist ".hgignore"
  assert_file_not_exist ".hgtags"
  assert_file_not_exist "_darcs"
  assert_file_not_exist "_darcs/format"
}

@test "Test 4: No flags, archive without vcs files" {
  run bash -c "$EXECUTABLE -x -C . -f ../excluded.tar"
  assert_success

  assert_file_exist "file"
  assert_dir_exist "dir"
  assert_file_not_exist "CVS"
  assert_file_not_exist "CVS/fileattr"
  assert_file_not_exist ".cvsignore"
  assert_file_not_exist "RCS"
  assert_file_not_exist "RCS/somefile"
  assert_file_not_exist "SCCS"
  assert_file_not_exist "SCCS/somefile"
  assert_file_not_exist ".svn"
  assert_file_not_exist ".svn/format"
  assert_file_not_exist ".git"
  assert_file_not_exist ".git/config"
  assert_file_not_exist ".gitignore"
  assert_file_not_exist ".gitattributes"
  assert_file_not_exist ".gitmodules"
  assert_file_not_exist ".arch-ids"
  assert_file_not_exist ".arch-ids/somefile"
  assert_file_not_exist "{arch}"
  assert_file_not_exist "{arch}/somefile"
  assert_file_not_exist "=RELEASE-ID"
  assert_file_not_exist "=meta-update"
  assert_file_not_exist "=update"
  assert_file_not_exist ".bzr"
  assert_file_not_exist ".bzr/checkout"
  assert_file_not_exist ".bzrignore"
  assert_file_not_exist ".bzrtags"
  assert_file_not_exist ".hg"
  assert_file_not_exist ".hg/dirstate"
  assert_file_not_exist ".hgignore"
  assert_file_not_exist ".hgtags"
  assert_file_not_exist "_darcs"
  assert_file_not_exist "_darcs/format"
}
