# Contributing to ClockOR

Thank you for considering contributing to ClockOR!

## Development Setup

1. Install [Rust](https://rustup.rs/) (stable toolchain)
2. Clone the repository:
   ```bash
   git clone https://github.com/imonoonoko/ClockOR.git
   cd ClockOR
   ```
3. Build and run:
   ```bash
   cargo run
   ```

## Coding Guidelines

- Run `cargo fmt` before committing
- Ensure `cargo clippy -- -D warnings` passes with no warnings
- Add tests for new functionality where possible
- Keep commits focused and well-described

## Pull Request Process

1. Fork the repository and create a feature branch
2. Make your changes
3. Run the full check suite:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   cargo build --release
   ```
4. Open a pull request against the `master` branch
5. Describe your changes clearly in the PR description

## Bug Reports

When filing a bug report, please include:

- Windows version
- ClockOR version
- Steps to reproduce the issue
- Expected vs actual behavior
- Screenshots if applicable
