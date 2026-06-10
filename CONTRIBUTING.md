# Contributing to mure

Thank you for your interest in contributing to mure, a command-line tool for creating and managing multiple repositories.

## Development Setup

```bash
git clone https://github.com/kitsuyui/mure.git
cd mure
lefthook install   # install pre-commit and pre-push hooks
```

The project uses [lefthook](https://lefthook.dev/) to run the same checks as CI locally.

## Running Tests and Checks

```bash
cargo test                      # run tests
cargo clippy -- -D warnings     # lint
cargo fmt --all -- --check      # format check
```

Or let the hooks run automatically when you commit/push.

## Submitting a Pull Request

1. Fork the repository and create a topic branch from `main`.
2. Make your changes. Keep each PR focused on one change.
3. Ensure all tests and clippy checks pass.
4. Open a pull request against `main` using the provided template.

## Reporting Bugs

Use the bug report issue template. Include your OS, mure version, and steps to reproduce.

## Security Vulnerabilities

See [SECURITY.md](SECURITY.md) for the disclosure process. Do not open a public issue with exploit details.

## Code Style

This project follows standard Rust formatting (`cargo fmt`). Run `cargo fmt` before committing.

## License

By contributing, you agree that your contributions will be licensed under the BSD-3-Clause license.
