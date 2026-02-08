#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_copy.c
#
# Creates files with names of various lengths (1..200 chars) along with
# hardlinks, symlinks, and directories. Archives and extracts them, then
# verifies everything roundtrips correctly.

load '../test_helper.bash'

# --allow-unsafe-links: the test creates symlinks targeting ../f/... which
# point outside the extraction directory. bsdtar allows these by default.
EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite --allow-unsafe-links"
LOOP_MAX=200

# Generate a filename of length $1 matching the pattern from the C test:
# repeating "abcdefghijklmnopqrstuvwxyz..." with a "_NNN" suffix.
generate_filename() {
  local len=$1
  if [ "$len" -eq 1 ]; then
    echo "1"
    return
  fi
  if [ "$len" -eq 2 ]; then
    echo "a2"
    return
  fi
  local result=""
  for ((j=0; j<len; j++)); do
    local c=$(( j % 26 ))
    result+=$(printf "\\x$(printf '%02x' $((97 + c)))")
  done
  # Overwrite trailing chars with _NNN
  local numstr="$len"
  local numlen=${#numstr}
  local pos=$((len - 1))
  for ((k=numlen-1; k>=0; k--)); do
    result="${result:0:$pos}${numstr:$k:1}${result:$((pos+1))}"
    pos=$((pos - 1))
  done
  result="${result:0:$pos}_${result:$((pos+1))}"
  echo "$result"
}

# Cross-platform device:inode identifier for hardlink verification
file_id() {
  if stat --version >/dev/null 2>&1; then
    stat -c '%d:%i' "$1"
  else
    stat -f '%d:%i' "$1"
  fi
}

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  umask 0

  assert_make_dir original 0775
  assert_make_dir original/f 0775
  assert_make_dir original/l 0775
  assert_make_dir original/m 0775
  assert_make_dir original/s 0775
  assert_make_dir original/d 0775

  for ((i=1; i<LOOP_MAX; i++)); do
    fname=$(generate_filename "$i")

    assert_make_file "original/f/$fname" 0777 "f/$fname"
    assert_make_hardlink "original/l/$fname" "original/f/$fname"
    assert_make_hardlink "original/m/$fname" "original/f/$fname"
    assert_make_symlink "original/s/$fname" "../f/$fname"
    assert_make_dir "original/d/$fname" 0775
  done

  # Create archive
  run bash -c "$EXECUTABLE -c -f archive -C original f d l m s >pack.out 2>pack.err"
  assert_success
  assert_file_empty pack.err
  assert_file_empty pack.out

  # Extract
  mkdir -p extracted
  run bash -c "$EXECUTABLE -x -f archive -C extracted >unpack.out 2>unpack.err"
  assert_success
  assert_file_empty unpack.err
  assert_file_empty unpack.out
}

teardown_file() {
  popd || exit 1
}

# Test 1: Verify files roundtrip correctly (default format).
@test "Test 1: Files roundtrip with default format" {
  cd "$BATS_FILE_TMPDIR"
  local errors=0
  for ((i=1; i<LOOP_MAX; i++)); do
    fname=$(generate_filename "$i")
    if [ ! -f "extracted/f/$fname" ]; then
      echo "MISSING: f/$fname" >&2
      errors=$((errors + 1))
    else
      expected="f/$fname"
      actual=$(cat "extracted/f/$fname")
      if [ "$actual" != "$expected" ]; then
        echo "CONTENT MISMATCH: f/$fname" >&2
        errors=$((errors + 1))
      fi
    fi
  done
  assert_equal "$errors" "0"
}

# Test 2: Verify hardlinks roundtrip correctly.
@test "Test 2: Hardlinks roundtrip" {
  cd "$BATS_FILE_TMPDIR"
  local errors=0
  for ((i=1; i<LOOP_MAX; i++)); do
    # C test: if (i + 2 <= limit)
    if ((i + 2 > LOOP_MAX)); then
      continue
    fi
    fname=$(generate_filename "$i")
    local id_f
    id_f=$(file_id "extracted/f/$fname")
    if [ ! -f "extracted/l/$fname" ]; then
      echo "MISSING: l/$fname" >&2
      errors=$((errors + 1))
    else
      local id_l
      id_l=$(file_id "extracted/l/$fname")
      if [ "$id_f" != "$id_l" ]; then
        echo "NOT HARDLINK: l/$fname" >&2
        errors=$((errors + 1))
      fi
    fi
    if [ ! -f "extracted/m/$fname" ]; then
      echo "MISSING: m/$fname" >&2
      errors=$((errors + 1))
    else
      local id_m
      id_m=$(file_id "extracted/m/$fname")
      if [ "$id_f" != "$id_m" ]; then
        echo "NOT HARDLINK: m/$fname" >&2
        errors=$((errors + 1))
      fi
    fi
  done
  assert_equal "$errors" "0"
}

# Test 3: Verify symlinks roundtrip correctly.
@test "Test 3: Symlinks roundtrip" {
  cd "$BATS_FILE_TMPDIR"
  local errors=0
  for ((i=1; i<LOOP_MAX; i++)); do
    fname=$(generate_filename "$i")
    target="../f/$fname"
    # C test: if (strlen(name2) <= limit)
    if ((${#target} > LOOP_MAX)); then
      continue
    fi
    if [ ! -L "extracted/s/$fname" ]; then
      echo "MISSING SYMLINK: s/$fname" >&2
      errors=$((errors + 1))
    else
      actual=$(readlink "extracted/s/$fname")
      if [ "$actual" != "$target" ]; then
        echo "SYMLINK MISMATCH: s/$fname -> $actual (expected $target)" >&2
        errors=$((errors + 1))
      fi
    fi
  done
  assert_equal "$errors" "0"
}

# Test 4: Verify directories roundtrip correctly.
@test "Test 4: Directories roundtrip" {
  cd "$BATS_FILE_TMPDIR"
  local errors=0
  for ((i=1; i<LOOP_MAX; i++)); do
    fname=$(generate_filename "$i")
    if [ ! -d "extracted/d/$fname" ]; then
      echo "MISSING DIR: d/$fname" >&2
      errors=$((errors + 1))
    fi
  done
  assert_equal "$errors" "0"
}

# Test 5: Verify no unexpected files in extracted directories.
@test "Test 5: No unexpected files in extracted directories" {
  cd "$BATS_FILE_TMPDIR"
  local errors=0
  for dir in d f l m s; do
    local count=0
    # C test: l/m/d use strlen(p) < limit, f/s use strlen(p) < limit + 1
    local max_len
    case "$dir" in
      f|s) max_len=$((LOOP_MAX + 1)) ;;
      *)   max_len=$LOOP_MAX ;;
    esac
    for entry in "extracted/$dir"/*; do
      [ -e "$entry" ] || continue
      count=$((count + 1))
      local name
      name=$(basename "$entry")
      local len=${#name}
      if ((len < 1 || len >= max_len)); then
        echo "INVALID LENGTH: $dir/$name (len=$len)" >&2
        errors=$((errors + 1))
      fi
      local expected_fname
      expected_fname=$(generate_filename "$len")
      if [ "$name" != "$expected_fname" ]; then
        echo "NAME MISMATCH: $dir/$name (expected $expected_fname)" >&2
        errors=$((errors + 1))
      fi
    done
    if [ "$count" -ne 199 ]; then
      echo "COUNT MISMATCH in $dir/: $count (expected 199)" >&2
      errors=$((errors + 1))
    fi
  done
  assert_equal "$errors" "0"
}

