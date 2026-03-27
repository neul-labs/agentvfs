# Contributing to AgentVFS

Thank you for your interest in contributing to AgentVFS! This document provides guidelines and information for contributors.

## Code of Conduct

Please be respectful and constructive in all interactions. We welcome contributors of all experience levels.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git

### Setting Up Development Environment

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/agentvfs.git
   cd agentvfs
   ```

3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/neul-labs/agentvfs.git
   ```

4. Build the project:
   ```bash
   cargo build
   ```

5. Run tests:
   ```bash
   cargo test
   ```

### Building with Optional Features

```bash
# Build with Sled backend
cargo build --features sled-backend

# Build with FUSE support
cargo build --features fuse

# Build with all features
cargo build --all-features
```

## Development Workflow

### Branching Strategy

- `main` - Stable release branch
- `develop` - Development branch for next release
- `feature/*` - Feature branches
- `fix/*` - Bug fix branches

### Making Changes

1. Create a feature branch from `develop`:
   ```bash
   git checkout develop
   git pull upstream develop
   git checkout -b feature/your-feature-name
   ```

2. Make your changes, following the code style guidelines

3. Run the test suite:
   ```bash
   cargo test
   cargo test --features sled-backend
   ```

4. Run clippy and fix any warnings:
   ```bash
   cargo clippy --all-targets --all-features
   ```

5. Format your code:
   ```bash
   cargo fmt
   ```

6. Commit your changes with a descriptive message:
   ```bash
   git commit -m "Add feature: description of your changes"
   ```

7. Push and create a pull request:
   ```bash
   git push origin feature/your-feature-name
   ```

## Code Style Guidelines

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Address all `cargo clippy` warnings
- Write documentation for public APIs
- Use descriptive variable and function names

### Documentation

- Document all public functions, structs, and modules
- Include examples in doc comments where helpful
- Keep README and documentation up to date with changes

### Testing

- Write unit tests for new functionality
- Add integration tests for CLI commands
- Ensure tests pass on all platforms (Linux, macOS, Windows)

## Pull Request Process

1. **Title**: Use a clear, descriptive title
2. **Description**: Explain what your PR does and why
3. **Tests**: Include tests for new functionality
4. **Documentation**: Update docs if needed
5. **Changelog**: Add entry to CHANGELOG.md

### PR Checklist

- [ ] Tests pass (`cargo test --all-features`)
- [ ] No clippy warnings (`cargo clippy --all-features`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated

## Adding New Features

### Adding a New Command

1. Create a new file in `src/commands/`
2. Implement the command logic
3. Register the command in `src/commands/mod.rs`
4. Add CLI arguments in `src/main.rs`
5. Add tests
6. Update documentation

### Adding a New Storage Backend

1. Create a new file in `src/storage/` (e.g., `lmdb.rs`)
2. Implement the `StorageBackend` trait
3. Implement all backend-specific methods (see `sqlite.rs` for reference)
4. Add the feature flag to `Cargo.toml`
5. Export the backend in `src/storage/mod.rs`
6. Add tests for the new backend
7. Update documentation

## Reporting Issues

### Bug Reports

Please include:
- AgentVFS version (`avfs --version`)
- Operating system and version
- Steps to reproduce the issue
- Expected vs actual behavior
- Relevant error messages or logs

### Feature Requests

Please include:
- Clear description of the feature
- Use cases and motivation
- Any implementation ideas (optional)

## Questions and Discussions

- Use GitHub Discussions for questions
- Use GitHub Issues for bug reports and feature requests

## License

By contributing to AgentVFS, you agree that your contributions will be licensed under the MIT License.

## Thank You!

Your contributions help make AgentVFS better for everyone. We appreciate your time and effort!
