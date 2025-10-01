# Temporary Directories for Golden Tests

This directory tree contains scratch space used by the compatibility test
harnesses. The `.gitignore` makes sure generated archives and working copies
are not committed. Harnesses should create their own subdirectories under
`tests/tmp` and remove them after execution.
