# Portable Network Archive — Bats Test Suite

This project uses [Bats (Bash Automated Testing System)](https://github.com/bats-core/bats-core) to validate core functionality through shell-based integration tests. Tests are located under `tests/bats/`.

## Dependencies

The test suite relies on the following Bats libraries:

- [bats-core](https://github.com/bats-core/bats-core)
- [bats-support](https://github.com/bats-core/bats-support)
- [bats-assert](https://github.com/bats-core/bats-assert)
- [bats-file](https://github.com/ztombol/bats-file)

These libraries will be installed automatically using the setup script.

## Setup

Clone the repository and run the provided setup script to install test dependencies:

If you're using Ubuntu, you can install bats-core via apt:

```bash
sudo apt-get update
sudo apt-get install bats
```

```bash
./setup.sh
```

This will download specific versions of required Bats extensions under `tests/bats/lib/`.

## Running Tests

To run the full test suite:

```bash
cd tests/bats
bats .
```

To run a specific test file:

```bash
bats large_file.bats
```

Ensure that all test files are executable (`chmod +x <file>.bats` if needed).

---

For more details on writing tests in Bats, refer to the [official documentation](https://github.com/bats-core/bats-core).