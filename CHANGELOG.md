# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of async-retry library
- Core retry functionality with `Retry` builder
- Three backoff strategies: `FixedDelay`, `ExponentialBackoff`, `FibonacciBackoff`
- Conditional retry support via `with_condition()`
- Maximum duration limiting via `with_max_duration()`
- Runtime-agnostic sleep via feature flags
- `tokio-timer` feature for Tokio runtime support
- `async-std-timer` feature for async-std runtime support
- `jitter` feature for randomized backoff delays
- `logging` feature for integrated logging support
- Comprehensive documentation and examples
- Full test coverage including unit tests, integration tests, and doctests

### Changed
- N/A (initial release)

### Deprecated
- N/A (initial release)

### Removed
- N/A (initial release)

### Fixed
- N/A (initial release)

### Security
- N/A (initial release)

## [0.1.0] - YYYY-MM-DD

### Added
- Initial public release
- Basic retry functionality
- Backoff strategies
- Conditional retries
- Maximum duration support
- Runtime-agnostic timer support
- Comprehensive documentation

[Unreleased]: https://github.com/Jacques-Murray/async-retry/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Jacques-Murray/async-retry/releases/tag/v0.1.0
