load 'lib/bats-support/load'
load 'lib/bats-assert/load'
load 'lib/bats-file/load'

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
