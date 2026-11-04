BASE_DIR=$(dirname "${BASH_SOURCE[0]}")
load "$BASE_DIR/lib/bats-support/load"
load "$BASE_DIR/lib/bats-assert/load"
load "$BASE_DIR/lib/bats-file/load"

# Link count function (Linux, macOS, FreeBSD)
file_link_count() {
  local file=$1

  if [ ! -e "$file" ]; then
    echo "0"
    return
  fi

  if stat --version >/dev/null 2>&1; then
    # GNU stat (Linux)
    stat -c %h "$file"
  else
    # BSD stat (macOS, FreeBSD)
    stat -f %l "$file"
  fi
}

# Assert that a file has the expected number of hard links
assert_link_count() {
  local expected=$1
  local file=$2
  local actual

  actual=$(file_link_count "$file")
  assert_equal "$actual" "$expected" "Expected $file to have $expected hard links, but got $actual"
}

# Mtime function (Linux, macOS, FreeBSD)
file_mtime() {
  local file=$1

  if stat --version >/dev/null 2>&1; then
    # GNU stat (Linux)
    stat -c %Y "$file"
  else
    # BSD stat (macOS, FreeBSD)
    stat -f %m "$file"
  fi
}

# Set file mtime to epoch seconds (Linux, macOS, FreeBSD)
set_file_mtime() {
  local epoch=$1
  local file=$2

  if stat --version >/dev/null 2>&1; then
    # GNU touch (Linux)
    touch -d "@$epoch" "$file"
  else
    # BSD touch (macOS, FreeBSD)
    TZ=UTC touch -t "$(date -u -r "$epoch" +%Y%m%d%H%M.%S)" "$file"
  fi
}

# Create file and assert that exists
assert_make_file() {
  local path=$1
  local mode=$2
  local contents=$3

  echo -n "$contents" >"$path"
  chmod "$mode" "$path"
  assert_file_exists "$path"
}

# Create directory and assert that exists
assert_make_dir() {
  local path=$1
  local mode=$2
  local contents=$3

  mkdir -m "$mode" "$path"
  assert_dir_exists "$path"
}

# Create hardlink and assert that exists
assert_make_hardlink() {
  local link_path=$1
  local target_path=$2
  ln "$target_path" "$link_path"
  assert_file_exists "$link_path"
}

# Create symlink and assert that exists
assert_make_symlink() {
  local link_path=$1
  local target_path=$2
  local is_dir=$3 # use this on windows # TODO: Support windows
  ln -s "$target_path" "$link_path"
  assert_symlink_to "$target_path" "$link_path"
}
