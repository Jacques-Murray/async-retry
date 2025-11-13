# Contributing to async-retry

Thank you for your interest in contributing to async-retry! This document provides guidelines and instructions for contributing.

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to follow. Please be respectful and constructive in all interactions.

## How to Contribute

### Reporting Bugs

If you find a bug, please create an issue with:
- A clear, descriptive title
- Steps to reproduce the bug
- Expected behavior
- Actual behavior
- Your environment (Rust version, OS, async runtime)
- Minimal code example that demonstrates the issue

### Suggesting Enhancements

Enhancement suggestions are welcome! Please create an issue with:
- A clear, descriptive title
- Detailed description of the proposed feature
- Use cases and examples
- Any potential drawbacks or alternatives you've considered

### Pull Requests

1. **Fork the repository** and create your branch from `main`
2. **Make your changes**:
   - Follow the existing code style
   - Add tests for new functionality
   - Update documentation as needed
   - Ensure all tests pass
3. **Commit your changes**:
   - Use clear, descriptive commit messages
   - Reference any related issues
4. **Push to your fork** and submit a pull request

## Development Setup

### Prerequisites

- Rust 1.75 or later (MSRV)
- Cargo

### Building

```bash
# Build with Tokio timer
cargo build --features tokio-timer

# Build with async-std timer
cargo build --features async-std-timer

# Build with all optional features
cargo build --all-features
```

### Testing

```bash
# Run tests with Tokio
cargo test --features tokio-timer

# Run tests with async-std
cargo test --features async-std-timer

# Run tests with all features
cargo test --all-features

# Run doctests
cargo test --doc --features tokio-timer
```

### Running Examples

```bash
cargo run --example simple_retry --features tokio-timer
cargo run --example conditional_retry --features tokio-timer
```

### Linting

```bash
# Check formatting
cargo fmt --check

# Run Clippy
cargo clippy --all-features -- -D warnings
```

## Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use meaningful variable and function names
- Add documentation comments for public APIs
- Keep functions focused and concise
- Prefer explicit error handling over panicking

### Documentation Style

- Public items should have doc comments (`///`)
- Include examples in doc comments where helpful
- Explain the "why" as well as the "what"
- Use proper markdown formatting
- Reference related items with backticks and brackets: [`Retry`], [`Backoff`]

### Testing Guidelines

- Write unit tests for individual functions
- Write integration tests for end-to-end scenarios  
- Test edge cases and error conditions
- Use descriptive test names that explain what is being tested
- Keep tests focused on a single concern

Example test structure:

```rust
#[tokio::test]
async fn test_retry_succeeds_on_third_attempt() {
    // Setup
    let op = create_flaky_operation(succeed_on: 3);
    let strategy = FixedDelay::new(Duration::from_millis(10)).take(5);
    
    // Execute
    let result = Retry::new(strategy, || op.run()).await;
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(op.attempts(), 3);
}
```

## Project Structure

```
async-retry/
├── src/
│   ├── lib.rs          # Main library entry point
│   ├── backoff.rs      # Backoff strategies
│   └── sleep.rs        # Runtime-agnostic sleep
├── tests/
│   └── integration_test.rs  # Integration tests
├── examples/
│   ├── simple_retry.rs
│   └── conditional_retry.rs
├── Cargo.toml
└── README.md
```

## Adding New Features

### New Backoff Strategy

1. Add the strategy struct in `src/backoff.rs`
2. Implement `Iterator<Item = Duration>`
3. Add builder methods if needed
4. Add unit tests
5. Update README.md with usage example
6. Add to public exports in `lib.rs` if appropriate

### New Retry Features

1. Consider if it fits the existing API
2. Discuss in an issue first for significant changes
3. Implement with minimal API surface
4. Add comprehensive tests
5. Update documentation

## Commit Message Guidelines

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests after the first line

Examples:
```
Add jitter support to backoff strategies

Implement Jitter wrapper for all backoff strategies to prevent
thundering herd problems.

Fixes #42
```

## Review Process

1. All pull requests require review before merging
2. Automated checks must pass (tests, linting)
3. Documentation must be updated for public API changes
4. Backwards compatibility should be maintained when possible
5. Breaking changes require a major version bump

## Release Process

Releases are managed by maintainers:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create a git tag
4. Publish to crates.io
5. Create GitHub release

## Questions?

If you have questions that aren't covered in this guide:
- Open an issue for discussion
- Check existing issues and pull requests
- Review the README.md and code documentation

## License

By contributing to async-retry, you agree that your contributions will be licensed under both the MIT License and Apache License 2.0.
