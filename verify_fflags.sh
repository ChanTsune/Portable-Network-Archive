#!/bin/bash
set -e

# Clean up previous run
rm -rf test_file test.pna out

# Create test file
touch test_file
chflags nodump test_file

# Archive
./target/debug/pna experimental stdio -c --fflags test_file > test.pna

# Extract
mkdir out
cd out
../target/debug/pna experimental stdio -x --fflags < ../test.pna

# Verify
if ls -lO test_file | grep -q "nodump"; then
    echo "Verification SUCCESS: 'nodump' flag preserved."
else
    echo "Verification FAILED: 'nodump' flag missing."
    ls -lO test_file
    exit 1
fi
